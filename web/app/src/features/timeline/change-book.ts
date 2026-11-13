import { useRef } from "react";

import type { KeptDates } from "./date-fields.ts";
import type { BarDays } from "./lines.ts";

/** What the screen knows of one issue whose dates are changing. */
export interface Changing {
  readonly dates: KeptDates;
  readonly days: BarDays;
  /** How many of its changes are on their way. */
  readonly count: number;
  /** The number of the newest change asked for: only its *Undo* is offered. */
  readonly newest: number;
}

/**
 * The book of the issues whose dates are changing on this screen, and of the bars a run of keys
 * holds: kept across draws, read and written only through these functions.
 */
export function useChangeBook() {
  const changing = useRef(new Map<string, Changing>());
  const held = useRef(new Set<string>());
  const offers = useRef(new Map<string, number>());
  return {
    /** The *Undo* on offer for the newest change of an issue, kept after its change ended. */
    offerOf: (id: string): number | undefined => offers.current.get(id),
    setOffer: (id: string, offer: number): void => {
      offers.current.set(id, offer);
    },
    get: (id: string): Changing | undefined => changing.current.get(id),
    put: (id: string, now: Changing): void => {
      changing.current.set(id, now);
    },
    forget: (id: string): void => {
      changing.current.delete(id);
    },
    holds: (id: string): boolean => held.current.has(id),
    hold: (id: string): void => {
      held.current.add(id);
    },
    release: (id: string): void => {
      held.current.delete(id);
    },
  };
}

/** The book, as the queue of changes reads it. */
export type ChangeBook = Readonly<ReturnType<typeof useChangeBook>>;
