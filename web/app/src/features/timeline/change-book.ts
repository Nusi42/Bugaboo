import { useState } from "react";

import type { KeptDates } from "./date-fields.ts";
import type { BarDays } from "./lines.ts";

/** Where an issue stands before a change: its bar on this screen, or the dates it keeps (the tray). */
export interface Before {
  readonly dates: KeptDates;
  readonly days: BarDays | undefined;
}

/** What the screen knows of one issue whose dates are changing. */
export interface Changing {
  readonly dates: KeptDates;
  readonly days: BarDays;
  /** How many of its changes are on their way. */
  readonly count: number;
  /** The number of the newest change asked for: only its *Undo* is offered. */
  readonly newest: number;
  /** The dates the server holds after the last of its changes that ended: what *Undo* puts back. */
  readonly saved: Before;
}

/**
 * The book of the issues whose dates are changing on this screen, and of the bars a run of keys
 * holds: read and written only through these functions.
 */
export function changeBook() {
  const changing = new Map<string, Changing>();
  const held = new Set<string>();
  const offers = new Map<string, number>();
  return {
    /** The *Undo* on offer for the newest change of an issue, kept after its change ended. */
    offerOf: (id: string): number | undefined => offers.get(id),
    setOffer: (id: string, offer: number): void => {
      offers.set(id, offer);
    },
    get: (id: string): Changing | undefined => changing.get(id),
    put: (id: string, now: Changing): void => {
      changing.set(id, now);
    },
    forget: (id: string): void => {
      changing.delete(id);
    },
    holds: (id: string): boolean => held.has(id),
    hold: (id: string): void => {
      held.add(id);
    },
    release: (id: string): void => {
      held.delete(id);
    },
  };
}

/** The book, as the queue of changes reads it. */
export type ChangeBook = Readonly<ReturnType<typeof changeBook>>;

/** The book, kept across draws of the screen. */
export function useChangeBook(): ChangeBook {
  const [book] = useState(changeBook);
  return book;
}
