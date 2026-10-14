type Listener = () => void;

/**
 * The people waiting for a change, for any state that a screen subscribes to
 * (`useSyncExternalStore`). Written once here; a store of the app and the appearance use it.
 */
export class Listeners {
  private readonly all = new Set<Listener>();

  /** How many are waiting. */
  get size(): number {
    return this.all.size;
  }

  /** Adds a listener; returns the way to stop. */
  readonly subscribe = (listener: Listener): (() => void) => {
    this.all.add(listener);
    return () => {
      this.all.delete(listener);
    };
  };

  /** Tells everybody that something changed. */
  readonly notify = (): void => {
    for (const listener of this.all) {
      listener();
    }
  };
}
