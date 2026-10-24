import type { Heartbeat, LockOutcome } from "@bugaboo/api-client";

import type { LostReason, Who } from "./long-text-model.ts";

/** The lock somebody else holds, as the server says it. */
export type HeldElsewhere = Extract<LockOutcome, { readonly kind: "held-by-other" }>;

/** The text and version a heartbeat keeps as the person's draft. */
export interface HeartbeatDraft {
  readonly text: string;
  readonly base_version: number;
}

/** What asking for the lock came to. */
export type Answer =
  | { readonly kind: "mine"; readonly token: string }
  | { readonly kind: "held"; readonly lock: HeldElsewhere }
  /** Another ask or the end of the session came meanwhile: the answer counts for nothing. */
  | { readonly kind: "stale" };

/** The calls a session makes, with the real client behind them or a double in a test. */
export interface SessionDeps {
  readonly take: (editor: string) => Promise<LockOutcome>;
  readonly editAnyway: (editor: string) => Promise<LockOutcome>;
  readonly askToFinish: () => Promise<void>;
  readonly heartbeat: (token: string, draft?: HeartbeatDraft) => Promise<Heartbeat>;
  readonly release: (token: string) => Promise<void>;
  /** Calls the function as often as the server wants a heartbeat until the returned function is called. */
  readonly startHeartbeats: (beat: () => Promise<unknown>) => () => void;
  readonly newEditorId: () => string;
}

export interface SessionOptions {
  /**
   * The draft every heartbeat keeps (a long text); `null` skips that beat. Without it a heartbeat
   * carries nothing (a layout).
   */
  readonly draft?: () => HeartbeatDraft | null;
  /** When the lease ran out: ask for the lock again (a long text) or call it lost (a layout). */
  readonly onRanOut: "take-again" | "lost";
  /** The lock is not the person's any more; `by` is who took it, or who holds it now. */
  readonly onLost: (reason: LostReason, by?: Who, problem?: unknown) => void;
  /** Somebody asked the person to finish. */
  readonly onAsked?: (from: Who) => void;
  /** The lease ran out and the lock is being asked for again. */
  readonly onRetaking?: () => void;
  /** The lock came back after the lease had run out. */
  readonly onRetaken?: () => void;
}

/**
 * One person's hold on one edit lock (ADR-0019), for every editor that has one. It owns the editor
 * ID, the token, the generation and the heartbeat, and keeps three rules:
 *
 * - Only the answer of the latest ask counts. A lock that an older ask took is let go, unless it
 *   is the token the latest ask holds: the server gives the same editor the same token again.
 * - Releases go out one after another, and every ask waits for the ones on their way: a release
 *   that arrives after the next ask would take that lock away.
 * - Nothing is released that was not taken: a lock that was taken over or ran out is only dropped.
 */
export class EditLockSession {
  private readonly editor: string;
  private held: string | undefined;
  private stopBeats: (() => void) | undefined;
  private current = 0;
  private asking = false;
  private releasing = 0;
  private queue: Promise<void> = Promise.resolve();
  /** Tokens that older asks took while a newer one was still open: judged when it answers. */
  private readonly strays = new Set<string>();

  private readonly deps: SessionDeps;
  private readonly options: SessionOptions;

  constructor(deps: SessionDeps, options: SessionOptions) {
    this.deps = deps;
    this.options = options;
    this.editor = deps.newEditorId();
  }

  /** The token of the lock the person holds. */
  get token(): string | undefined {
    return this.held;
  }

  /** Counts the asks and endings; code that awaited compares it to see it was overtaken. */
  get generation(): number {
    return this.current;
  }

  /** Asks for the lock. Rejects with the error of the call, only if no other ask came meanwhile. */
  readonly take = (): Promise<Answer> => this.ask(this.deps.take);

  /** *Edit anyway*: the lock becomes the person's. */
  readonly editAnyway = (): Promise<Answer> => this.ask(this.deps.editAnyway);

  /** *Ask Ana to finish*. */
  readonly askToFinish = (): Promise<void> => this.deps.askToFinish();

  private async ask(how: (editor: string) => Promise<LockOutcome>): Promise<Answer> {
    this.current += 1;
    const asked = this.current;
    this.asking = true;
    void this.release();
    try {
      if (this.releasing > 0) {
        await this.queue;
      }
      if (asked !== this.current) {
        return { kind: "stale" };
      }
      const outcome = await how(this.editor);
      if (asked !== this.current) {
        this.stray(outcome);
        return { kind: "stale" };
      }
      this.asking = false;
      if (outcome.kind === "held-by-other") {
        this.sweep();
        return { kind: "held", lock: outcome };
      }
      this.keep(outcome.lock_token);
      this.sweep();
      return { kind: "mine", token: outcome.lock_token };
    } catch (error) {
      if (asked !== this.current) {
        return { kind: "stale" };
      }
      this.asking = false;
      this.sweep();
      throw error;
    }
  }

  /** The answer of an ask that was overtaken: its lock is let go, if it is nobody's. */
  private stray(outcome: LockOutcome): void {
    if (outcome.kind !== "taken") {
      return;
    }
    if (this.asking) {
      this.strays.add(outcome.lock_token);
    } else if (outcome.lock_token !== this.held) {
      this.send(outcome.lock_token);
    }
  }

  /** Lets go of the tokens of overtaken asks, except the one that is held. */
  private sweep(): void {
    for (const token of this.strays) {
      if (token !== this.held) {
        this.send(token);
      }
    }
    this.strays.clear();
  }

  private keep(token: string): void {
    this.held = token;
    const generation = this.current;
    this.stopBeats = this.deps.startHeartbeats(() => this.beat(token, generation));
  }

  private async beat(token: string, generation: number): Promise<void> {
    if (this.held !== token) {
      return;
    }
    const draft = this.options.draft?.();
    if (draft === null) {
      return;
    }
    const answer = await this.deps.heartbeat(token, draft);
    if (generation !== this.current || this.held !== token) {
      return;
    }
    const asked = answer.notices.find((notice) => notice.kind === "finish-asked");
    if (asked !== undefined) {
      this.options.onAsked?.(asked.from);
    }
    if (answer.lock.kind === "taken-over") {
      this.forget();
      this.options.onLost("taken-over", answer.lock.by);
    } else if (answer.lock.kind === "ran-out") {
      await this.ranOut();
    }
  }

  private async ranOut(): Promise<void> {
    if (this.options.onRanOut === "lost") {
      // A lock that ran out is no lock (ADR-0019): there is nothing to let go.
      this.forget();
      this.options.onLost("ran-out");
      return;
    }
    // The lease ran out (the tab slept): the lock is asked for again, if nobody has it.
    this.forget();
    this.options.onRetaking?.();
    let answer: Answer;
    try {
      answer = await this.take();
    } catch (problem) {
      // Only an ask that nothing overtook rejects: the lock could not be asked for. The text
      // stays, and asking again is up to the person.
      this.options.onLost("ran-out", undefined, problem);
      return;
    }
    if (answer.kind === "mine") {
      this.options.onRetaken?.();
    } else if (answer.kind === "held") {
      this.options.onLost("ran-out", answer.lock.holder);
    }
  }

  /** Stops the heartbeat and drops the token without letting go: it is not the person's (any more). */
  readonly forget = (): void => {
    this.stopBeats?.();
    this.stopBeats = undefined;
    this.held = undefined;
  };

  /** Stops the heartbeat and lets go of the lock that is held, if any; settles when the server answered. */
  readonly release = (): Promise<void> => {
    const token = this.held;
    this.forget();
    if (token !== undefined) {
      this.send(token);
    }
    return this.queue;
  };

  /**
   * Why a save was refused for the lock's sake: one heartbeat asks whether somebody took over. The
   * lock is given up first (beats stop, an ask on its way is overtaken and its lock let go).
   */
  readonly explain = async (reason: LostReason): Promise<{ reason: LostReason; by?: Who }> => {
    if (reason !== "ran-out") {
      return { reason };
    }
    this.current += 1;
    this.asking = false;
    const token = this.held;
    this.forget();
    try {
      if (token === undefined) {
        return { reason };
      }
      const answer = await this.deps.heartbeat(token);
      const from = answer.notices.find((notice) => notice.kind === "finish-asked")?.from;
      if (from !== undefined) {
        this.options.onAsked?.(from);
      }
      return answer.lock.kind === "taken-over"
        ? { reason: "taken-over", by: answer.lock.by }
        : { reason };
    } catch {
      return { reason };
    }
  };

  /**
   * A save was refused for the lock's sake. A text that changed meanwhile leaves the lock the
   * person's, but nothing renews it: it is let go, so others do not read *… is editing* for a
   * minute about a person who has stopped. A lock that ran out or was taken over is not theirs.
   */
  readonly refused = (reason: LostReason): Promise<void> => {
    if (reason === "version-changed") {
      return this.release();
    }
    this.forget();
    return Promise.resolve();
  };

  /** The editing ends: whatever answer is on its way counts for nothing, and the lock is let go. */
  readonly end = (): Promise<void> => {
    this.current += 1;
    this.asking = false;
    const done = this.release();
    this.sweep();
    return done;
  };

  /** Puts a release in the queue: one after another, and never a failure out of it. */
  private send(token: string): void {
    // With nothing on its way the call goes out at once.
    const run =
      this.releasing === 0
        ? this.deps.release(token)
        : this.queue.then(() => this.deps.release(token));
    this.releasing += 1;
    this.queue = run.catch(ignore).finally(() => {
      this.releasing -= 1;
    });
  }
}

/** The end of a call whose answer nobody waits for. */
function ignore(): void {
  // Nothing to do: a release that failed is ended by the lease.
}
