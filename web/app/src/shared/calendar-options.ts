/**
 * The calendar of a view (UX design section 4.16): the date field the issues stand on and a month or a
 * week, how an address says them and how a saved search keeps them. A calendar has no groups and no
 * columns; everything else of a view is in `list-view.ts`.
 */

import type { Schema } from "@bugaboo/api-client";

import { listWord } from "./list-columns.ts";
import type { SearchState } from "./search-address.ts";

/** The options of a calendar: the date field the issues stand on, and a month or a week. */
export type CalendarOptions = Schema<"CalendarOptionsBody">;

/** What a calendar shows first: a month or a week (*Month · Week*). */
export type SpanWord = Schema<"CalendarSpan">;

/** Every span of a calendar, the same way, in the order they are offered. */
const spans: Readonly<Record<SpanWord, true>> = { month: true, week: true };

function isSpan(word: string | undefined): word is SpanWord {
  return word !== undefined && Object.hasOwn(spans, word);
}

/** The two spans by their words, so a screen compares with a name and not with a text. */
export const spanOf = { month: "month", week: "week" } as const satisfies Record<
  SpanWord,
  SpanWord
>;

/** The spans of a calendar in the order they are offered. */
export const spanWords: readonly SpanWord[] = Object.keys(spans).filter((word) => isSpan(word));

/** The calendar a saved search has when it says nothing of one: *Due date*, by the month. */
export const builtInCalendar: CalendarOptions = { dates_field: null, span: "month" };

/** The calendar an address asks for: what it says, and the defaults for what it leaves out. */
export function calendarOf(state: SearchState, defaults: CalendarOptions): CalendarOptions {
  const field = state.from ?? defaults.dates_field;
  return {
    // *Due date* is the view's own when it names no field (`null`).
    dates_field: field === listWord.dueDate ? null : field,
    span: isSpan(state.span) ? state.span : defaults.span,
  };
}

/** The parts of the address that say how a calendar differs from its defaults. */
export function calendarParts(
  calendar: CalendarOptions,
  usual: CalendarOptions,
): Pick<SearchState, "span" | "from"> {
  return {
    ...(calendar.span === usual.span ? {} : { span: calendar.span }),
    // Back to *Due date* from a saved view of another field is written too.
    ...(calendar.dates_field === usual.dates_field
      ? {}
      : { from: calendar.dates_field ?? listWord.dueDate }),
  };
}
