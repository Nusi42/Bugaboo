import { startingState, type Editing, type LongTextDeps, type Phase } from "./long-text-model.ts";

type Listener = () => void;

/**
 * What a screen subscribes to: the state of an editing, the lock token and the timer that goes with
 * it. The rules of editing are in `LongTextEditor`, which extends this.
 */
export abstract class EditingStore {
  protected state: Editing;
  private readonly listeners = new Set<Listener>();
  protected token: string | null = null;
  protected stopBeats: (() => void) | null = null;
  protected readonly deps: LongTextDeps;

  protected constructor(
    deps: LongTextDeps,
    source: { readonly saved: string; readonly version: number },
  ) {
    this.deps = deps;
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

  protected set(change: Partial<Editing>): void {
    this.state = { ...this.state, ...change };
    for (const listener of this.listeners) {
      listener();
    }
  }

  /** The phase as it is now, for code that has awaited. */
  protected phaseNow(): Phase {
    return this.state.phase;
  }

  protected stopHeartbeats(): void {
    this.stopBeats?.();
    this.stopBeats = null;
  }
}
