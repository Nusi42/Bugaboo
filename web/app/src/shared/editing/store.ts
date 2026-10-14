type Listener = () => void;

/** What a screen subscribes to: a state that is replaced whenever something changed. */
export abstract class Store<State> {
  protected state: State;
  private readonly listeners = new Set<Listener>();

  protected constructor(first: State) {
    this.state = first;
  }

  /** The state now: a new object whenever something changed. */
  readonly snapshot = (): State => this.state;

  readonly subscribe = (listener: Listener): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  protected replace(next: State): void {
    this.state = next;
    for (const listener of this.listeners) {
      listener();
    }
  }
}
