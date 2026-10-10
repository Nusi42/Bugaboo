import { conflictOf, type LockOutcome } from "@bugaboo/api-client";

import { codeOf } from "../errors.ts";
import {
  lostReasonOf,
  newestDraft,
  startingState,
  type Editing,
  type LongTextDeps,
  type Phase,
} from "./long-text-model.ts";

export type * from "./long-text-model.ts";

/**
 * Editing a long text without ever losing it (ISS-17, ADR-0019): the lock on the first keystroke,
 * the heartbeat that keeps it, the draft in the browser and on the server, and what the screen
 * must say when something goes wrong. It is a plain controller with its dependencies passed in,
 * so every path can be tested without a browser; `useLongText` puts it on a screen.
 */

type Listener = () => void;

/** The controller of one long text of one record. */
export class LongTextEditor {
  private state: Editing;
  private readonly listeners = new Set<Listener>();
  private token: string | null = null;
  private stopBeats: (() => void) | null = null;
  private baseVersion: number;
  private readonly editor: string;
  private saveWhenLocked = false;

  private readonly deps: LongTextDeps;

  constructor(deps: LongTextDeps, source: { readonly saved: string; readonly version: number }) {
    this.deps = deps;
    this.baseVersion = source.version;
    this.editor = deps.newEditorId();
    this.state = startingState(source);
  }

  /** The state now: a new object whenever something changed. */
  readonly snapshot = (): Editing => this.state;

  readonly subscribe = (listener: Listener): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  private set(change: Partial<Editing>): void {
    this.state = { ...this.state, ...change };
    for (const listener of this.listeners) {
      listener();
    }
  }

  /** The record was read again: the text and version now. A box that is not open follows it. */
  readonly setSource = (source: { readonly saved: string; readonly version: number }): void => {
    const reading = this.state.phase.kind === "reading";
    this.set({
      saved: source.saved,
      version: source.version,
      ...(reading ? { text: source.saved, startedFrom: source.saved } : {}),
    });
    if (reading) {
      this.baseVersion = source.version;
    }
  };

  /** Opens the box. A draft of the person is looked for and offered. */
  readonly open = (): void => {
    if (this.state.phase.kind !== "reading") {
      return;
    }
    this.baseVersion = this.state.version;
    this.set({ ...startingState(this.state), phase: { kind: "open" } });
    void this.findDraft();
  };

  private async findDraft(): Promise<void> {
    const newest = newestDraft(
      [this.deps.browserDraft.read(), await this.deps.serverDraft().catch(() => null)],
      this.state.saved,
    );
    // Only while nothing was typed: a person who started writing has their own text.
    if (newest !== undefined && this.state.phase.kind === "open") {
      this.set({ offered: newest });
    }
  }

  /** Continues from the offered draft; the first key of a draft that differs takes the lock. */
  readonly continueDraft = (): void => {
    const draft = this.state.offered;
    if (draft !== null) {
      this.set({ offered: null });
      this.type(draft.text);
    }
  };

  /** Starts again from the saved text and drops the offered draft. */
  readonly dropDraft = (): void => {
    this.set({ offered: null });
    this.deps.browserDraft.forget();
    void this.deps.throwAwayServerDraft().catch(ignore);
  };

  /** The text in the box changed. The first change takes the lock. */
  readonly type = (text: string): void => {
    const { phase } = this.state;
    if (phase.kind === "reading" || phase.kind === "saving") {
      return;
    }
    this.set({ text });
    this.deps.browserDraft.write(text, this.baseVersion);
    if (phase.kind === "open" && text !== this.state.saved) {
      void this.begin();
    }
  };

  private async begin(): Promise<void> {
    this.set({ phase: { kind: "taking" }, problem: null });
    try {
      this.arrive(await this.deps.take(this.editor));
    } catch (error) {
      this.set({ phase: { kind: "open" }, problem: error });
    }
  }

  /** The answer to asking for the lock (or to *Edit anyway*). */
  private arrive(outcome: LockOutcome): void {
    if (outcome.status === "held") {
      this.stopHeartbeats();
      this.set({ phase: { kind: "held", lock: outcome } });
      return;
    }
    this.token = outcome.lock_token;
    this.set({ phase: { kind: "locked" } });
    this.stopBeats ??= this.deps.startHeartbeats(this.beat);
    if (this.saveWhenLocked) {
      this.saveWhenLocked = false;
      void this.save();
    }
  }

  private readonly beat = async (): Promise<void> => {
    const token = this.token;
    if (token === null || this.state.phase.kind !== "locked") {
      return;
    }
    const answer = await this.deps.heartbeat(token, {
      text: this.state.text,
      base_version: this.baseVersion,
    });
    const asked = answer.notices.find((notice) => notice.kind === "finish_asked");
    if (asked !== undefined) {
      this.set({ asked: asked.from });
    }
    if (answer.lock.status === "taken_over") {
      this.stopHeartbeats();
      this.set({ phase: { kind: "lost", reason: "taken-over", by: answer.lock.by } });
    } else if (answer.lock.status === "ran_out") {
      // The lease ran out (the tab slept): take the lock again if nobody has it.
      this.stopHeartbeats();
      this.token = null;
      this.set({ phase: { kind: "taking" } });
      this.arrive(await this.deps.take(this.editor));
      const now = this.phaseNow();
      if (now.kind === "held") {
        this.set({ phase: { kind: "lost", reason: "ran-out", by: now.lock.holder } });
      }
    }
  };

  /** The phase as it is now, for code that has awaited. */
  private phaseNow(): Phase {
    return this.state.phase;
  }

  private stopHeartbeats(): void {
    this.stopBeats?.();
    this.stopBeats = null;
  }

  /** Saves the text. Without a lock it takes one first (after *Compare and save*). */
  readonly save = async (): Promise<void> => {
    const { phase, text } = this.state;
    if (phase.kind === "open") {
      this.cancel();
      return;
    }
    if (phase.kind === "lost" || phase.kind === "held") {
      this.saveWhenLocked = true;
      await this.begin();
      return;
    }
    const token = this.token;
    if (phase.kind !== "locked" || token === null) {
      return;
    }
    this.set({ phase: { kind: "saving" }, problem: null });
    // A change of another field since the start is no conflict: the text is still the one the
    // person started from, so the save goes on the version now. A changed text is one.
    const version =
      this.state.saved === this.state.startedFrom ? this.state.version : this.baseVersion;
    try {
      await this.deps.save({ text, version, lockToken: token });
    } catch (error) {
      await this.failed(error);
      return;
    }
    this.finish();
    await this.deps.refresh().catch(ignore);
  };

  private async failed(error: unknown): Promise<void> {
    const reason = lostReasonOf(codeOf(error));
    if (reason === undefined) {
      // Not the lock's business (the network, a rule): the text stays in the box, still locked.
      this.set({ phase: { kind: "locked" }, problem: error });
      return;
    }
    this.stopHeartbeats();
    this.token = null;
    const by = conflictOf(error)?.changed_by;
    this.set({
      phase: { kind: "lost", reason, ...(by === null || by === undefined ? {} : { by }) },
      problem: null,
    });
    if (reason === "version-changed") {
      await this.deps.refresh().catch(ignore);
    }
  }

  /** The text is saved or thrown away: nothing is held and no draft is kept. */
  private finish(): void {
    this.stopHeartbeats();
    this.token = null;
    this.deps.browserDraft.forget();
    this.set({ phase: { kind: "reading" }, offered: null, asked: null, problem: null });
  }

  /** Throws the editing away: the lock is let go and no draft is kept. */
  readonly cancel = (): void => {
    const token = this.token;
    this.finish();
    this.set({ text: this.state.saved, startedFrom: this.state.saved });
    void this.deps.throwAwayServerDraft().catch(ignore);
    if (token !== null) {
      void this.deps.release(token).catch(ignore);
    }
  };

  /** *Compare and save*: the person's choice of the text becomes the text, and is saved on the newest version. */
  readonly resolve = async (text: string): Promise<void> => {
    this.baseVersion = this.state.version;
    this.set({ text, startedFrom: this.state.saved });
    this.deps.browserDraft.write(text, this.baseVersion);
    await this.save();
  };

  /** *Ask Ana to finish*. */
  readonly askToFinish = async (): Promise<void> => {
    try {
      await this.deps.askToFinish();
    } catch (error) {
      this.set({ problem: error });
    }
  };

  /** *Edit anyway*: the other person's text stays their draft. */
  readonly editAnyway = async (): Promise<void> => {
    if (this.state.phase.kind !== "held") {
      return;
    }
    try {
      this.arrive(await this.deps.editAnyway(this.editor));
    } catch (error) {
      this.set({ problem: error });
    }
  };

  /** The screen goes away: the beats stop and the lock is let go; the draft stays. */
  readonly dispose = (): void => {
    const token = this.token;
    this.stopHeartbeats();
    this.token = null;
    if (token !== null) {
      void this.deps.release(token).catch(ignore);
    }
  };
}

/** The end of a call whose answer nobody waits for. */
function ignore(): void {
  // Nothing to do: a failure of a release or of throwing a draft away is tried again by the lease.
}
