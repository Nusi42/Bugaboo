import { browserSchedule, undoWindowMs, type Schedule } from "./optimistic.ts";

/** One action that can still be taken back. */
export interface UndoOffer {
  readonly id: number;
  /** What happened, in a sentence: "The invitation was taken back." */
  readonly message: string;
  /** Puts things back as they were. */
  readonly undo: () => void;
}

/** What a caller gives: the sentence and the way back. */
export interface UndoRequest {
  readonly message: string;
  readonly undo: () => void;
}

/**
 * The actions that can be undone for another 10 seconds (UX §6: reversible actions happen at once
 * and offer *Undo* instead of asking "Are you sure?"). The shell shows them; anything that can be
 * taken back offers itself through `offer`.
 */
export interface UndoStore {
  readonly offer: (request: UndoRequest) => number;
  /** Takes the action back, if it is still on offer; returns whether it was. */
  readonly take: (id: number) => boolean;
  readonly list: () => readonly UndoOffer[];
  readonly subscribe: (listener: () => void) => () => void;
}

/** Makes a store. The clock is a parameter so tests do not have to wait 10 seconds. */
export function createUndoStore(schedule: Schedule = browserSchedule): UndoStore {
  const listeners = new Set<() => void>();
  let offers: readonly UndoOffer[] = [];
  let nextId = 1;
  const stops = new Map<number, () => void>();
  const changed = (next: readonly UndoOffer[]): void => {
    offers = next;
    for (const listener of listeners) {
      listener();
    }
  };
  const drop = (id: number): void => {
    stops.get(id)?.();
    stops.delete(id);
    changed(offers.filter((offer) => offer.id !== id));
  };
  return {
    offer: (request) => {
      const id = nextId;
      nextId += 1;
      stops.set(
        id,
        schedule(() => {
          drop(id);
        }, undoWindowMs),
      );
      changed([...offers, { id, ...request }]);
      return id;
    },
    take: (id) => {
      const found = offers.find((offer) => offer.id === id);
      if (found === undefined) {
        return false;
      }
      drop(id);
      found.undo();
      return true;
    },
    list: () => offers,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/** The one store of the app. */
export const undoStore = createUndoStore();
