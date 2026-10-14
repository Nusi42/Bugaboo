import { conflictOf } from "@bugaboo/api-client";

import { codeOf } from "../errors.ts";
import { EditLockSession, type Answer } from "./edit-lock-session.ts";
import { lostReasonOf, startingState, type LongTextDeps } from "./long-text-model.ts";
import { ignore } from "./long-text-store.ts";
import { WatchingEditor } from "./long-text-watching.ts";

export type * from "./long-text-model.ts";

/**
 * Editing a long text without ever losing it (ISS-17, ADR-0019): the lock on the first keystroke,
 * the heartbeat that keeps it, the draft in the browser and on the server, and what the screen
 * must say when something goes wrong. It is a plain controller with its dependencies passed in,
 * so every path can be tested without a browser; `useLongText` puts it on a screen.
 */

/** The controller of one long text of one record. */
export class LongTextEditor extends WatchingEditor {
  private baseVersion: number;
  private saveWhenLocked = false;
  protected readonly lock = new EditLockSession(this.deps, {
    draft: () =>
      this.state.phase.kind === "locked"
        ? { text: this.state.text, base_version: this.baseVersion }
        : null,
    onRanOut: "take-again",
    onAsked: (from) => {
      this.set({ asked: from });
    },
    onRetaking: () => {
      this.set({ phase: { kind: "taking" } });
    },
    onRetaken: () => {
      this.arrive();
    },
    onLost: (reason, by, problem) => {
      this.saveWhenLocked = false;
      this.set({
        phase: { kind: "lost", reason, ...(by === undefined ? {} : { by }) },
        ...(problem === undefined ? {} : { problem }),
      });
    },
  });

  constructor(deps: LongTextDeps, source: { readonly saved: string; readonly version: number }) {
    super(deps, source);
    this.baseVersion = source.version;
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
    void this.findHolder();
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
      void this.begin(this.lock.take);
    }
  };

  /** Asks for the lock (or takes it with *Edit anyway*) and takes the answer, unless it was overtaken. */
  private async begin(how: () => Promise<Answer>): Promise<void> {
    this.set({ phase: { kind: "taking" }, problem: null });
    try {
      this.answered(await how());
    } catch (error) {
      this.saveWhenLocked = false;
      this.set({ phase: { kind: "open" }, problem: error });
    }
  }

  private answered(answer: Answer): void {
    if (answer.kind === "held") {
      this.saveWhenLocked = false;
      this.stopHeartbeats();
      this.set({ phase: { kind: "held", lock: answer.lock } });
    } else if (answer.kind === "mine") {
      this.arrive();
    }
  }

  /** The lock is the person's (the session beats for it): whatever watched a holder gives way. */
  private arrive(): void {
    this.set({ phase: { kind: "locked" } });
    this.stopHeartbeats();
    if (this.saveWhenLocked) {
      this.saveWhenLocked = false;
      void this.save();
    }
  }

  /** Saves the text. Without a lock it takes one first (after *Compare and save*). */
  readonly save = async (): Promise<void> => {
    const { phase, text } = this.state;
    if (phase.kind === "open") {
      this.cancel();
      return;
    }
    if (phase.kind === "taking") {
      // Save was pressed before the answer to asking for the lock came: save when it does.
      this.saveWhenLocked = true;
      return;
    }
    if (phase.kind === "lost" || phase.kind === "held") {
      this.saveWhenLocked = true;
      await this.begin(this.lock.take);
      return;
    }
    const token = this.lock.token;
    if (phase.kind !== "locked" || token === undefined) {
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
    void this.lock.refused(reason);
    const by = conflictOf(error)?.changed_by;
    this.set({
      phase: { kind: "lost", reason, ...(by === null || by === undefined ? {} : { by }) },
      problem: null,
    });
    // Whatever the reason, the newer text and version are what Compare and save works with.
    await this.deps.refresh().catch(ignore);
  }

  /** The text is saved or thrown away: nothing is held and no draft is kept. */
  private finish(): void {
    this.saveWhenLocked = false;
    this.lock.forget();
    this.deps.browserDraft.forget();
    this.set({ phase: { kind: "reading" }, offered: null, asked: null, problem: null });
  }

  /** Throws the editing away: the lock is let go and no draft is kept. */
  readonly cancel = (): void => {
    void this.lock.end();
    this.finish();
    this.set({ text: this.state.saved, startedFrom: this.state.saved });
    void this.deps.throwAwayServerDraft().catch(ignore);
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
      await this.lock.askToFinish();
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
      this.answered(await this.lock.editAnyway());
    } catch (error) {
      this.set({ problem: error });
    }
  };

  /** The screen goes away: the beats stop and the lock is let go; the draft stays. */
  readonly dispose = (): void => {
    void this.lock.end();
    this.stopHeartbeats();
  };
}
