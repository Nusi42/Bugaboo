import { t } from "@bugaboo/messages";

import { dayInSentence } from "../../shared/clock.ts";
import { askAboutConflict, askAboutClash, type ConflictAsker } from "../../shared/editing/index.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { known } from "../../shared/present.ts";
import type { Refused } from "../../shared/refused.ts";
import { idOf, refreshCalendar, type CalendarEntry } from "./calendar-data.ts";
import type { DateWords } from "./date-words.ts";
import { takeVersion } from "./save-day.ts";

/** Everything `settle` works with. */
export interface Ways {
  /** The issue is where the server has it again, unless a newer move of it has put it somewhere else. */
  readonly clear: (id: string, day: string) => void;
  /** Keeps the *Undo* on offer for an issue, by its ID. */
  readonly keepOffer: (id: string, offer: number) => void;
  readonly asker: ConflictAsker;
  /** The change on its way: the issue stands on a day at once, and the one before it is no longer offered. */
  readonly apply: (entry: CalendarEntry, from: string, day: string) => void;
  readonly words: DateWords;
}

/** A move: where the issue stood, and where it went. */
export interface Days {
  readonly from: string;
  readonly day: string;
}

/** The offer to undo a move that was saved. */
function offerUndo(entry: CalendarEntry, days: Days, ways: Ways): void {
  ways.keepOffer(
    idOf(entry),
    undoStore.offer({
      message: t("calendar.moved", {
        key: known(entry.issue.readable_id) ?? "",
        field: ways.words.noun,
        day: dayInSentence(days.day),
      }),
      undo: () => {
        ways.apply(entry, days.day, days.from);
      },
    }),
  );
}

/**
 * What happened to a move: *Undo* when it was saved, the reason where the offers to undo stand when
 * the server refused, or the clash that a change of somebody else made, which asks *Keep their change*
 * or *Use mine* over the page; until it is answered the issue stays where the person put it (UX §6.4).
 */
export function settle(
  entry: CalendarEntry,
  days: Days,
  refused: Refused | null,
  ways: Ways,
): void {
  const id = idOf(entry);
  const clear = (): void => {
    ways.clear(id, days.day);
  };
  if (refused === null) {
    void refreshCalendar().then(clear);
    offerUndo(entry, days, ways);
  } else if ("reason" in refused) {
    clear();
    undoStore.offer({ message: refused.reason });
  } else {
    const { conflict } = refused;
    askAboutClash(
      conflict,
      {
        keepTheirs: () => {
          askAboutConflict(null);
          void refreshCalendar().then(clear);
        },
        useMine: () => {
          askAboutConflict(null);
          takeVersion(entry, conflict.version ?? 0);
          ways.apply(entry, entry.day, days.day);
        },
      },
      ways.asker,
    );
  }
}
