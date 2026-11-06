import type { Schema } from "@bugaboo/api-client";
import type { MessageKey } from "@bugaboo/messages";

import { momentOfClock } from "../../shared/clock.ts";
import { addDays } from "../../shared/format.ts";

type WorkDay = Schema<"WorkDayBody">;
type Span = Schema<"SpanBody">;
/** One of the person's time entries, as the server answers it. */
export type TimeEntry = Schema<"TimeEntryItem">;

/** The breaks typed in a cell: each from a time of day to a time of day (minutes after midnight). */
export type BreakTimes = readonly (readonly [number, number])[];

/** The cells of the work-day rows a person types into. *Working hours* is worked out by the server. */
export type WorkDayField = "start" | "end" | "breaks";

/** The periods and breaks of a day, as the server takes them. */
export interface Times {
  readonly periods: readonly Span[];
  readonly breaks: readonly Span[];
}

/** What a typed cell of the work day comes to: new times of the day, or why it cannot be made. */
export type PlannedDay =
  | { readonly times: Times }
  | { readonly problem: MessageKey }
  /** A start for a day that cannot run, kept (minutes after midnight) until its end is typed. */
  | { readonly held: number };

export interface WorkDayEdit {
  /** The day as the server has it; none for a date nobody has recorded. */
  readonly day: WorkDay | undefined;
  readonly date: string;
  /** The zone the clock of the day is read in: the zone the day started in, or the person's. */
  readonly zone: string;
  readonly field: WorkDayField;
  /** A time of day (minutes after midnight) for *Start* and *End*; not used for *Breaks*. */
  readonly minutes: number;
  /** Whether the day may run without an end: only today's does. */
  readonly canRun: boolean;
  /** For *Breaks*: the times of the breaks typed, as clock times; the person gives times, never a total. */
  readonly breaks?: BreakTimes | undefined;
  /** The start typed before for a day with no record, waiting for its end. */
  readonly held?: number | undefined;
}

/** The moment the clock reads `minutes` on `date`, or on the next morning when that is not after `start`. */
function endAfter(start: string, date: string, minutes: number, zone: string): string {
  const sameDay = momentOfClock(date, minutes, zone);
  return new Date(sameDay).getTime() > new Date(start).getTime()
    ? sameDay
    : momentOfClock(addDays(date, 1), minutes, zone);
}

/**
 * The times of a day once `field` holds what was typed. The browser only places what was typed on
 * the day; whether it is allowed, what it adds up to and which rule it breaks is the server's.
 * Breaks are typed as times (`12:10-12:40`, several with commas) and replace the breaks of the
 * day; the browser never invents a break, and the history of the day keeps the old times.
 */
export function planWorkDayEdit(edit: WorkDayEdit): PlannedDay {
  const { day, date, zone, field, minutes, canRun, held } = edit;
  const periods = day?.periods ?? [];
  const first = periods[0];
  const last = periods.at(-1);
  if (field === "start") {
    if (first === undefined) {
      const start = momentOfClock(date, minutes, zone);
      return canRun
        ? { times: { periods: [{ start, end: null }], breaks: [] } }
        : { held: minutes };
    }
    const start = momentOfClock(date, minutes, zone);
    return {
      times: { periods: [{ ...first, start }, ...periods.slice(1)], breaks: day?.breaks ?? [] },
    };
  }
  if (field === "end" && first === undefined && held !== undefined) {
    const start = momentOfClock(date, held, zone);
    const end = endAfter(start, date, minutes, zone);
    return { times: { periods: [{ start, end }], breaks: [] } };
  }
  if (first === undefined || last === undefined) {
    return { problem: "time.sheet.start-first" };
  }
  if (field === "end") {
    const end = endAfter(last.start, date, minutes, zone);
    return {
      times: { periods: [...periods.slice(0, -1), { ...last, end }], breaks: day?.breaks ?? [] },
    };
  }
  const breaks = (edit.breaks ?? []).map(([from, to]) => {
    const start = momentOfClock(date, from, zone);
    return { start, end: endAfter(start, date, to, zone) };
  });
  return { times: { periods, breaks } };
}

/** What a typed cell of an issue does to the entries of that issue on that day. */
export type PlannedCell =
  | { readonly kind: "none" }
  | { readonly kind: "several" }
  | {
      readonly kind: "add";
      readonly issue: string;
      readonly date: string;
      readonly minutes: number;
    }
  | {
      readonly kind: "change";
      readonly entry: string;
      readonly version: number;
      readonly minutes: number;
    }
  | { readonly kind: "delete"; readonly entry: string; readonly version: number };

export interface IssueCellEdit {
  readonly issue: string;
  readonly date: string;
  /** The entries of the person for that issue on that day. */
  readonly entries: readonly TimeEntry[];
  /** The length typed, or none when the cell was cleared. */
  readonly minutes: number | undefined;
}

/** Books, changes or takes back the time of a cell. A cell of several entries is changed in their details. */
export function planIssueCell(edit: IssueCellEdit): PlannedCell {
  const { entries, minutes } = edit;
  const [only, ...more] = entries;
  if (more.length > 0) {
    return { kind: "several" };
  }
  if (only === undefined) {
    return minutes === undefined
      ? { kind: "none" }
      : { kind: "add", issue: edit.issue, date: edit.date, minutes };
  }
  if (minutes === undefined) {
    return { kind: "delete", entry: only.id, version: only.version };
  }
  return minutes === only.minutes
    ? { kind: "none" }
    : { kind: "change", entry: only.id, version: only.version, minutes };
}

/**
 * Whether a change of a work day asks for a reason before it is saved (ADR-0035, decision 7): any
 * change of another person's day, and a change of one's own day from the next day on. The server
 * decides again and refuses a change without one.
 */
export function reasonIsNeeded(change: {
  readonly own: boolean;
  readonly date: string;
  readonly today: string;
}): boolean {
  return !change.own || change.date < change.today;
}

function sameSpans(a: readonly Span[], b: readonly Span[]): boolean {
  return (
    a.length === b.length &&
    a.every(
      (span, at) => span.start === b[at]?.start && (span.end ?? null) === (b[at]?.end ?? null),
    )
  );
}

/** Whether the day has these times already, so typing them again changes nothing. */
export function hasTimes(day: WorkDay | undefined, times: Times): boolean {
  return (
    day !== undefined &&
    sameSpans(day.periods, times.periods) &&
    sameSpans(day.breaks, times.breaks)
  );
}
