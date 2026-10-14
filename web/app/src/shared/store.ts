import { Listeners } from "@bugaboo/ui";

/** What a screen subscribes to: a state that is replaced whenever something changed. */
export interface StateCell<State> {
  /** The state now: a new object whenever something changed. */
  readonly snapshot: () => State;
  readonly subscribe: (listener: () => void) => () => void;
  /** Replaces the state and tells everybody. */
  readonly replace: (next: State) => void;
}

/** A state with its listeners, for a module that keeps one state for the whole app. */
export function createStore<State>(first: State): StateCell<State> {
  let state = first;
  const listeners = new Listeners();
  return {
    snapshot: () => state,
    subscribe: listeners.subscribe,
    replace: (next) => {
      state = next;
      listeners.notify();
    },
  };
}

/** A `StateCell` for a class whose methods change its state. */
export abstract class Store<State> {
  private readonly cell: StateCell<State>;

  protected constructor(first: State) {
    this.cell = createStore(first);
  }

  protected get state(): State {
    return this.cell.snapshot();
  }

  readonly snapshot = (): State => this.cell.snapshot();

  readonly subscribe = (listener: () => void): (() => void) => this.cell.subscribe(listener);

  protected replace(next: State): void {
    this.cell.replace(next);
  }
}
