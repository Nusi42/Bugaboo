import { useEffect, useRef, useState } from "react";

import { conflictAsker } from "../../shared/editing/index.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import type { DateField } from "../issues/index.ts";
import { idOf, type CalendarEntry } from "./calendar-data.ts";
import type { DateWords } from "./date-words.ts";
import { saveDay } from "./save-day.ts";
import { settle } from "./settle-move.ts";

/** What the screen needs to move an issue: the field it stands on, and how it is called. */
export interface MoveInput {
  readonly field: DateField | undefined;
  readonly words: DateWords;
}

/** The moves of issues to other days. */
export interface DayMoves {
  /** The days issues are shown on while a move of them is on its way, by the ID of the issue. */
  readonly shown: Readonly<Record<string, string>>;
  /** Moves an issue to a day: it stands there at once, and the change is saved. */
  readonly move: (entry: CalendarEntry, day: string) => void;
  /** The day an issue stands on now: where the server has it, or where a move on its way put it. */
  readonly dayOf: (entry: CalendarEntry) => string;
}

/**
 * The moves of issues to other days (PLAN-09): the issue is on its new day at once, the change is saved
 * and the calendar read again, with *Undo*, the reason of the server, or the clash of a change somebody
 * made first (`settle-move.ts`). Nothing but the moved issue is drawn again in another place.
 */
export function useDayMoves({ field, words }: MoveInput): DayMoves {
  const [shown, setShown] = useState<Readonly<Record<string, string>>>({});
  // What the screen asks goes with it; words that are ready after it went are not shown.
  const [asker] = useState(conflictAsker);
  useEffect(() => {
    asker.stay();
    return asker.leave;
  }, [asker]);
  const offers = useRef(new Map<string, number>());
  const clear = (id: string, day: string): void => {
    setShown((before) => {
      const { [id]: now, ...others } = before;
      return now === day ? others : before;
    });
  };
  const apply = (entry: CalendarEntry, from: string, day: string): void => {
    if (field === undefined) {
      return;
    }
    const offered = offers.current.get(idOf(entry));
    if (offered !== undefined) {
      undoStore.withdraw(offered);
    }
    setShown((before) => ({ ...before, [idOf(entry)]: day }));
    void saveDay(entry, field, day).then((refused) => {
      settle(entry, { from, day }, refused, {
        clear,
        keepOffer: (id, offer) => {
          offers.current.set(id, offer);
        },
        asker,
        apply,
        words,
      });
    });
  };
  const dayOf = (entry: CalendarEntry): string => shown[idOf(entry)] ?? entry.day;
  return {
    shown,
    dayOf,
    move: (entry, day) => {
      if (day !== dayOf(entry)) {
        apply(entry, dayOf(entry), day);
      }
    },
  };
}
