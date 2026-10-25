import { browserSchedule, undoWindowMs, type Schedule } from "./optimistic.ts";
import { createStore } from "./store.ts";

/** One action that can still be taken back. */
export interface UndoOffer {
  readonly id: number;
  /** What happened, in a sentence: "The invitation was taken back." */
  readonly message: string;
  /** The name of a file in the sentence: it is drawn whole, never broken at a hyphen. */
  readonly name?: string;
  /** Puts things back as they were. A notice that only tells something has none. */
  readonly undo?: () => void;
  /** Where the focus goes after `undo`: the thing that is back. Without it, the content heading. */
  readonly restoredFocus?: () => HTMLElement | null;
  /** The screen shows *Undo* where the change happened (a row of the Details), not at the bottom. */
  readonly inline?: boolean;
}

/** What a caller gives: the sentence and the way back. */
export interface UndoRequest {
  readonly message: string;
  readonly name?: string;
  readonly undo?: () => void;
  readonly restoredFocus?: () => HTMLElement | null;
  readonly inline?: boolean;
}

/**
 * The actions that can be undone for another 10 seconds (UX §6: reversible actions happen at once
 * and offer *Undo* instead of asking "Are you sure?"). The shell shows them; anything that can be
 * taken back offers itself through `offer`.
 */
export interface UndoStore {
  /** Stops counting down while the offer is pointed at or has the focus. */
  readonly pause: (id: number) => void;
  /** Goes on counting down with the time that was left. */
  readonly resume: (id: number) => void;
  /** The latest offer, for the shortcut. */
  readonly latest: () => UndoOffer | undefined;
  readonly offer: (request: UndoRequest) => number;
  /** Takes the action back, if it is still on offer; returns whether it was. */
  readonly take: (id: number) => boolean;
  readonly list: () => readonly UndoOffer[];
  readonly subscribe: (listener: () => void) => () => void;
}

/** The time each offer has left, which stops counting while the offer is held. */
interface Countdowns {
  /** Starts counting the whole window for an offer. */
  readonly start: (id: number, onElapsed: () => void) => void;
  readonly pause: (id: number) => void;
  readonly resume: (id: number) => void;
  /** Forgets an offer. */
  readonly stop: (id: number) => void;
}

function createCountdowns(schedule: Schedule, now: () => number): Countdowns {
  const stops = new Map<number, () => void>();
  const left = new Map<number, { remaining: number; since: number | null }>();
  const callbacks = new Map<number, () => void>();
  const count = (id: number, remaining: number): void => {
    left.set(id, { remaining, since: now() });
    stops.set(
      id,
      schedule(() => {
        callbacks.get(id)?.();
      }, remaining),
    );
  };
  return {
    start: (id, onElapsed) => {
      callbacks.set(id, onElapsed);
      count(id, undoWindowMs);
    },
    pause: (id) => {
      const state = left.get(id);
      if (state?.since !== null && state !== undefined) {
        stops.get(id)?.();
        stops.delete(id);
        const spent = now() - (state.since ?? 0);
        left.set(id, { remaining: Math.max(0, state.remaining - spent), since: null });
      }
    },
    resume: (id) => {
      const state = left.get(id);
      if (state !== undefined && state.since === null) {
        count(id, state.remaining);
      }
    },
    stop: (id) => {
      stops.get(id)?.();
      stops.delete(id);
      left.delete(id);
      callbacks.delete(id);
    },
  };
}

/** Makes a store. The clock is a parameter so tests do not have to wait 10 seconds. */
export function createUndoStore(
  schedule: Schedule = browserSchedule,
  now: () => number = Date.now,
): UndoStore {
  const countdowns = createCountdowns(schedule, now);
  const offers = createStore<readonly UndoOffer[]>([]);
  let nextId = 1;
  const drop = (id: number): void => {
    countdowns.stop(id);
    offers.replace(offers.snapshot().filter((offer) => offer.id !== id));
  };
  return {
    offer: (request) => {
      const id = nextId;
      nextId += 1;
      countdowns.start(id, () => {
        drop(id);
      });
      offers.replace([...offers.snapshot(), { id, ...request }]);
      return id;
    },
    pause: countdowns.pause,
    resume: countdowns.resume,
    latest: () => offers.snapshot().findLast((offer) => offer.undo !== undefined),
    take: (id) => {
      const found = offers.snapshot().find((offer) => offer.id === id);
      if (found === undefined) {
        return false;
      }
      drop(id);
      found.undo?.();
      return true;
    },
    list: offers.snapshot,
    subscribe: offers.subscribe,
  };
}

/** The one store of the app. */
export const undoStore = createUndoStore();
