import type { EditLockSession } from "./edit-lock-session.ts";
import { startingState, type Editing, type LongTextDeps } from "./long-text-model.ts";
import { Store } from "./store.ts";

/**
 * What a screen subscribes to: the state of an editing, and the timer that watches a holder. The
 * lock is the session's. The rules of editing are in `LongTextEditor`, which extends this.
 */
export abstract class EditingStore extends Store<Editing> {
  protected stopBeats: (() => void) | null = null;
  protected readonly deps: LongTextDeps;
  /** The hold on the lock: its token, its heartbeat, its generation. */
  protected abstract readonly lock: EditLockSession;

  protected constructor(
    deps: LongTextDeps,
    source: { readonly saved: string; readonly version: number },
  ) {
    super(startingState(source));
    this.deps = deps;
  }

  protected set(change: Partial<Editing>): void {
    this.replace({ ...this.state, ...change });
  }

  /** Stops the timer that watches a holder. */
  protected stopHeartbeats(): void {
    this.stopBeats?.();
    this.stopBeats = null;
  }
}

/** The end of a call whose answer nobody waits for. */
export function ignore(): void {
  // Nothing to do: a failure of a release or of throwing a draft away is tried again by the lease.
}
