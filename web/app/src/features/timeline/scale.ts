import { t } from "@bugaboo/messages";
import type { ScaleUnit, TokenName } from "@bugaboo/ui";

import {
  dayOfMonth,
  daysBetween,
  isWeekend,
  monthName,
  monthsLater,
  weekdayName,
  weekFirstOf,
} from "../../shared/clock.ts";
import { addDays, dayNumbersOf, formatCalendarDay } from "../../shared/format.ts";
import type { ZoomWord } from "../../shared/list-view.ts";
import type { FirstDay } from "../../shared/region.ts";

/**
 * The scale of the timeline (UX §4.7): which days the chart shows at each zoom, the units of its
 * scale, and where a day stands on it. It holds no rule of the plan: the bars come from the server
 * (ADR-0030, amendment M8), and this file only says where a day is drawn.
 */

/** The width of one day at each zoom, a token of the base theme file. */
export const dayWidthOf: Readonly<Record<ZoomWord, TokenName>> = {
  day: "size-day-day",
  week: "size-day-week",
  month: "size-day-month",
  quarter: "size-day-quarter",
};

const daysPerWeek = 7;
const monthsPerQuarter = 3;

/**
 * How many weeks the chart shows at each zoom: six weeks of days, half a year of weeks, a year of
 * months and of quarters. The server answers a span of a year at most (`work_times::RANGE_DAYS_MAX`),
 * so no span here is longer than 52 weeks.
 */
const shownWeeks: Readonly<Record<ZoomWord, number>> = { day: 6, week: 26, month: 52, quarter: 52 };

/** How many parts of what the chart shows *Earlier* and *Later* step: a quarter of it. */
const stepParts = 4;

/** The days a chart shows: the first and the last, both included, and how many. */
export interface Span {
  readonly first: string;
  readonly last: string;
  readonly days: number;
}

/** The span of a zoom that starts on `first`. */
export function spanFrom(first: string, zoom: ZoomWord): Span {
  const days = shownWeeks[zoom] * daysPerWeek;
  return { first, last: addDays(first, days - 1), days };
}

/** How many months a day is after the first month of its quarter. */
function intoQuarter(day: string): number {
  return (dayNumbersOf(day)[1] - 1) % monthsPerQuarter;
}

/** The first day of the unit that holds a day, at each zoom (`firstDay` is the person's first day of the week). */
const unitFirst: Readonly<Record<ZoomWord, (day: string, firstDay: FirstDay) => string>> = {
  day: (day) => day,
  week: (day, firstDay) => weekFirstOf(day, firstDay),
  month: (day) => monthsLater(day, 0),
  quarter: (day) => monthsLater(day, -intoQuarter(day)),
};

/** The first day of the unit after the one that starts on `first`, at each zoom. */
const unitAfter: Readonly<Record<ZoomWord, (first: string, firstDay: FirstDay) => string>> = {
  day: (first) => addDays(first, 1),
  week: (first, firstDay) => addDays(weekFirstOf(first, firstDay), daysPerWeek),
  month: (first) => monthsLater(first, 1),
  quarter: (first) => monthsLater(first, monthsPerQuarter - intoQuarter(first)),
};

/** How many units before the one of today a chart starts, so today stands near the start with room before it. */
const unitsBefore: Readonly<Record<ZoomWord, number>> = { day: 7, week: 2, month: 1, quarter: 0 };

/** The first day a chart shows around `today`. */
export function firstAround(today: string, zoom: ZoomWord, firstDay: FirstDay): string {
  let first = unitFirst[zoom](today, firstDay);
  for (let back = 0; back < unitsBefore[zoom]; back += 1) {
    first = unitFirst[zoom](addDays(first, -1), firstDay);
  }
  return first;
}

/** The first day after a step *Earlier* (`-1`) or *Later* (`1`). */
export function stepped(first: string, zoom: ZoomWord, direction: -1 | 1): string {
  const step = Math.round((shownWeeks[zoom] * daysPerWeek) / stepParts);
  return addDays(first, direction * step);
}

/** Where a day stands on the chart: days after its first day. */
export function placeOf(span: Span, day: string): number {
  return daysBetween(span.first, day);
}

/** The name of a unit of the scale at each zoom, by its first day. */
const unitName: Readonly<Record<ZoomWord, (first: string) => string>> = {
  day: (first) =>
    t("timeline.scale-day", { weekday: weekdayName(first, "short"), day: dayOfMonth(first) }),
  week: (first) => formatCalendarDay(first),
  month: (first) =>
    t("timeline.scale-month", { month: monthName(first), year: dayNumbersOf(first)[0] }),
  quarter: (first) =>
    t("timeline.scale-quarter", {
      quarter: Math.floor((dayNumbersOf(first)[1] - 1) / monthsPerQuarter) + 1,
      year: dayNumbersOf(first)[0],
    }),
};

/** The units of the scale of a span: each a day, a week, a month or a quarter, cut at the ends of the span. */
export function scaleUnits(span: Span, zoom: ZoomWord, firstDay: FirstDay): readonly ScaleUnit[] {
  const units: ScaleUnit[] = [];
  let first = span.first;
  while (placeOf(span, first) < span.days) {
    const next = unitAfter[zoom](first, firstDay);
    const start = placeOf(span, first);
    units.push({
      key: first,
      start,
      length: Math.min(placeOf(span, next), span.days) - start,
      label: unitName[zoom](first),
    });
    first = next;
  }
  return units;
}

/** A stretch of days on the chart: where it starts and how many days it holds. */
export interface Stretch {
  readonly start: number;
  readonly length: number;
}

/** The weekends of a span: shaded at the zoom of days and of weeks only, where a day is wide enough to see. */
export function weekends(span: Span, zoom: ZoomWord): readonly Stretch[] {
  if (zoom !== "day" && zoom !== "week") {
    return [];
  }
  const found: Stretch[] = [];
  for (let at = 0; at < span.days; at += 1) {
    if (isWeekend(addDays(span.first, at))) {
      const last = found.at(-1);
      if (last !== undefined && last.start + last.length === at) {
        found[found.length - 1] = { start: last.start, length: last.length + 1 };
      } else {
        found.push({ start: at, length: 1 });
      }
    }
  }
  return found;
}
