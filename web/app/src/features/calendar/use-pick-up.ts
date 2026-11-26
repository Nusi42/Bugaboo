import { useState } from "react";

import { addDays } from "../../shared/format.ts";
import { idOf, type CalendarEntry } from "./calendar-data.ts";
import type { DayMoves } from "./use-day-moves.ts";

/** The issue picked up with a key, and the day it is carried over. */
export interface Picked {
  readonly entry: CalendarEntry;
  readonly day: string;
}

/** What the screen can do with an issue picked up with a key. */
export interface PickUp {
  readonly picked: Picked | undefined;
  /** The issue that is to have the focus once it stands on its new day. */
  readonly focusAfter: string | undefined;
  readonly focused: () => void;
  /** Whether an issue is the one that is carried. */
  readonly carries: (entry: CalendarEntry) => boolean;
  readonly pickUp: (entry: CalendarEntry) => void;
  /** Carries the issue over `days` more days. */
  readonly carry: (days: number) => void;
  readonly putDown: () => void;
  readonly putBack: () => void;
}

/**
 * Picking an issue up with a key, carrying it over the days with the arrow keys, and putting it down
 * (PLAN-09, TEST-A11Y): the same change as a drag, saved once when it is put down, with one *Undo*.
 * While it is carried the issue stays where it is and the day it is over shows where it would stand,
 * so nothing on the screen moves under the focus; once it is put down it stands on its new day and has
 * the focus again. *Escape* puts it back.
 */
export function usePickUp(moves: Pick<DayMoves, "dayOf" | "move">): PickUp {
  const [picked, setPicked] = useState<Picked | undefined>();
  const [focusAfter, setFocusAfter] = useState<string | undefined>();
  return {
    picked,
    focusAfter,
    focused: (): void => {
      setFocusAfter(undefined);
    },
    carries: (entry) => picked !== undefined && idOf(picked.entry) === idOf(entry),
    pickUp: (entry): void => {
      setPicked({ entry, day: moves.dayOf(entry) });
    },
    carry: (days): void => {
      setPicked((now) => (now === undefined ? now : { ...now, day: addDays(now.day, days) }));
    },
    putDown: (): void => {
      if (picked !== undefined) {
        moves.move(picked.entry, picked.day);
        setFocusAfter(idOf(picked.entry));
      }
      setPicked(undefined);
    },
    putBack: (): void => {
      setPicked(undefined);
    },
  };
}
