import { addDays, dayNumbersOf } from "../../shared/format.ts";
import { monthsLater, weekDaysOf, weekFirstOf } from "../../shared/clock.ts";
import type { SpanWord } from "../../shared/list-view.ts";
import type { FirstDay } from "../../shared/region.ts";

/**
 * The days a calendar shows (UX §4.16): a week of seven days, or the weeks that hold a month, from the
 * first day of the week of the person's *Region*. The server answers six weeks at most
 * (`CALENDAR_DAYS_MAX`); a month never needs more. No rule of the plan is in here: only which days
 * are drawn, and where *Earlier* and *Later* go.
 */

/** The days on screen: whole weeks, each a row of seven days. */
export interface Window {
  readonly weeks: readonly (readonly string[])[];
  /** The first and the last day on screen, which the server is asked for. */
  readonly first: string;
  readonly last: string;
  /** The first day of the month the calendar is of; the days around it are drawn quieter. Week: none. */
  readonly month: string | undefined;
}

/** The first day of the month of a day (`2026-10-16` is `2026-10-01`). */
function monthStart(day: string): string {
  const [year, month] = dayNumbersOf(day);
  return `${String(year).padStart(4, "0")}-${String(month).padStart(2, "0")}-01`;
}

/** The days of the weeks that hold the days from `first` to `last`. */
function weeksAround(
  first: string,
  last: string,
  firstDay: FirstDay,
): readonly (readonly string[])[] {
  const weeks: (readonly string[])[] = [];
  for (let start = weekFirstOf(first, firstDay); start <= last; start = addDays(start, 7)) {
    weeks.push(weekDaysOf(start));
  }
  return weeks;
}

/** The days a calendar shows around `anchor`, a day inside the week or the month it is of. */
export function windowOf(anchor: string, span: SpanWord, firstDay: FirstDay): Window {
  const [first, last, month] =
    span === "week"
      ? [anchor, anchor, undefined]
      : [monthStart(anchor), addDays(monthsLater(anchor, 1), -1), monthStart(anchor)];
  const weeks = weeksAround(first, last, firstDay);
  const [start = anchor] = weeks[0] ?? [];
  const end = weeks.at(-1)?.at(-1) ?? anchor;
  return { weeks, first: start, last: end, month };
}

/** The anchor after *Earlier* (`-1`) or *Later* (`1`): a week, or a month. */
export function steppedAnchor(anchor: string, span: SpanWord, direction: -1 | 1): string {
  return span === "week" ? addDays(anchor, direction * 7) : monthsLater(anchor, direction);
}

/** Whether a day is inside the month the calendar is of; every day of a week is. */
export function inMonth(window: Window, day: string): boolean {
  return window.month === undefined || monthStart(day) === window.month;
}
