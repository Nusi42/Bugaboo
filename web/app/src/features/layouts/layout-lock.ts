import type { Heartbeat, LockOutcome } from "@bugaboo/api-client";

/** Where the person stands with the edit lock on a layout (ADR-0019, ADR-0027 decision 6). */
export type LockState =
  | { readonly phase: "taking" }
  | { readonly phase: "mine"; readonly token: string }
  | {
      readonly phase: "held";
      /** Who is editing, as the person may see them. */
      readonly name: string;
      readonly since: string;
      readonly canEditAnyway: boolean;
    }
  | { readonly phase: "lost" };

/** The calls the lock makes, with the real client behind them or a double in a test. */
export interface LockDeps {
  readonly take: (editor: string) => Promise<LockOutcome>;
  readonly editAnyway: (editor: string) => Promise<LockOutcome>;
  readonly askToFinish: () => Promise<void>;
  readonly heartbeat: (token: string) => Promise<Heartbeat>;
  readonly release: (token: string) => Promise<void>;
  /** Calls the function as often as the server wants a heartbeat until the returned function is called. */
  readonly startHeartbeats: (beat: () => Promise<unknown>) => () => void;
  readonly newEditorId: () => string;
  /** How a person who holds the lock is named. */
  readonly nameOf: (holder: Extract<LockOutcome, { status: "held" }>["holder"]) => string;
}

const taking: LockState = { phase: "taking" };
const lost: LockState = { phase: "lost" };

/**
 * The state of the edit lock on a layout while its editor is open: taken when the editor opens,
 * kept with a heartbeat, let go when it closes. It is a plain object a screen subscribes to, so
 * every path is tested without a browser.
 */
export class LayoutLockController {
  private current: LockState = taking;
  private readonly listeners = new Set<() => void>();
  private readonly editor: string;
  private readonly deps: LockDeps;
  private stop: (() => void) | undefined;
  private token: string | undefined;
  private closed = false;
  private generation = 0;

  constructor(deps: LockDeps) {
    this.deps = deps;
    this.editor = deps.newEditorId();
  }

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  readonly snapshot = (): LockState => this.current;

  private set(next: LockState): void {
    this.current = next;
    for (const listener of this.listeners) {
      listener();
    }
  }

  /**
   * Asks for the lock; called when the editor opens and again by *Try again*. Every ask makes a
   * new generation, and only the answer of the latest one counts (as in the long text editor):
   * a lock an older ask took is let go, so no heartbeat runs for it and no token is left behind.
   */
  readonly start = (): void => {
    this.release();
    this.closed = false;
    this.set(taking);
    this.ask(this.deps.take);
  };

  /** Asks `how` for the lock and takes the answer if nobody asked again meanwhile. */
  private ask(how: (editor: string) => Promise<LockOutcome>): void {
    this.generation += 1;
    const asked = this.generation;
    void how(this.editor).then(
      (outcome) => {
        this.holds(outcome, asked);
      },
      () => {
        if (asked === this.generation && !this.closed) {
          this.set(lost);
        }
      },
    );
  }

  private holds(outcome: LockOutcome, asked: number): void {
    if (this.closed || asked !== this.generation) {
      if (outcome.status === "taken") {
        void this.deps.release(outcome.lock_token);
      }
      return;
    }
    if (outcome.status === "held") {
      this.set({
        phase: "held",
        name: this.deps.nameOf(outcome.holder),
        since: outcome.since,
        canEditAnyway: outcome.can_edit_anyway,
      });
      return;
    }
    const mine = outcome.lock_token;
    this.token = mine;
    this.set({ phase: "mine", token: mine });
    this.stop = this.deps.startHeartbeats(async () => {
      const answer = await this.deps.heartbeat(mine);
      if (answer.lock.status !== "held" && !this.closed && asked === this.generation) {
        this.release();
        this.set(lost);
      }
    });
  }

  private release(): void {
    this.stop?.();
    this.stop = undefined;
    const held = this.token;
    this.token = undefined;
    if (held !== undefined) {
      void this.deps.release(held);
    }
  }

  /** The editor is closed: the heartbeat stops and the lock is let go. */
  readonly dispose = (): void => {
    this.closed = true;
    this.generation += 1;
    this.release();
  };

  /** *Edit anyway*: the lock becomes the caller's. */
  readonly editAnyway = (): void => {
    this.ask(this.deps.editAnyway);
  };

  readonly askToFinish = (): void => {
    void this.deps.askToFinish();
  };
}
