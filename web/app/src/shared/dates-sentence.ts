import type { Schema } from "@bugaboo/api-client";
import type { Wording } from "@bugaboo/wasm";

import { dateField } from "./date-fields.ts";
import { whoOf, type ChangeWords } from "./change-sentence.ts";
import { formatCalendarDay } from "./format.ts";
import { renderSaid } from "./said.ts";

type Item = Schema<"HistoryEntryItem">;

/** A first and a last day as the calendar writes them (`2026-11-03`). */
export interface Days {
  readonly start: string;
  readonly due: string;
}

/** Both dates of an issue before and after one event. */
export interface DatesMove {
  readonly before: Days;
  readonly after: Days;
}

/** The day one side of the change of a date holds, as the calendar writes it. */
function dayOf(
  items: readonly Item[],
  field: string,
  side: "before" | "after",
): string | undefined {
  const value = items.find((item) => item.change?.field === field)?.change?.[side];
  return value?.shown === "typed" && typeof value.value === "string" ? value.value : undefined;
}

/** The two items of an event that change the dates, which a sentence of its own says. */
export function dateItems(items: readonly Item[]): readonly Item[] {
  return items.filter(
    (item) => item.change?.field === dateField.start || item.change?.field === dateField.due,
  );
}

/**
 * Both dates moving in one event (ADR-0043, decision 6), when each had a day before and has one now:
 * one sentence says it. An event that sets, clears or changes only one of them has none, and its
 * items are said one by one.
 */
export function datesMoveOf(items: readonly Item[]): DatesMove | undefined {
  const [startBefore, dueBefore, startAfter, dueAfter] = [
    dayOf(items, dateField.start, "before"),
    dayOf(items, dateField.due, "before"),
    dayOf(items, dateField.start, "after"),
    dayOf(items, dateField.due, "after"),
  ];
  return startBefore === undefined ||
    dueBefore === undefined ||
    startAfter === undefined ||
    dueAfter === undefined
    ? undefined
    : {
        before: { start: startBefore, due: dueBefore },
        after: { start: startAfter, due: dueAfter },
      };
}

/** The days of a range in the person's region. */
function worded(days: Days): Days {
  return { start: formatCalendarDay(days.start), due: formatCalendarDay(days.due) };
}

/**
 * The sentence of both dates moving in one event: *Lea moved the dates from … to …*. The words are
 * chosen by `bb-core`, the days are written for the person's region.
 */
export function datesSentence(wording: Wording, entry: ChangeWords, move: DatesMove): string {
  return renderSaid(
    wording.datesMovedSentence({
      who: whoOf(entry),
      before: worded(move.before),
      after: worded(move.after),
    }),
  );
}
