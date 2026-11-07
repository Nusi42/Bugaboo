import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { isSaid, type Said, type Wording } from "@bugaboo/wasm";

import { exampleClock, readClock, weekdayAndDate, zoneOfNewDay } from "../../shared/clock.ts";
import { formatNumber, formatTime } from "../../shared/format.ts";
import { lengthTyped } from "../../shared/length.ts";
import type { Region } from "../../shared/region.ts";
import { renderSaid } from "../../shared/said.ts";
import type { BreakTimes, WorkDayField } from "./edits.ts";
import type { CellRead } from "./TimeCell.tsx";

/** The example of a decimal length in the sentence of a mistake: 1.5 hours, written with the person's mark. */
const EXAMPLE_HOURS = 1.5;

/** The times of day that the sentences of a mistake show to type: one in the morning, one in the afternoon. */
const EXAMPLE_MORNING = { hour: 8, minute: 5 } as const;
const EXAMPLE_AFTERNOON = { hour: 17, minute: 15 } as const;

/** The break the sentences show to type: a lunch break on the person's clock. */
export function exampleBreak(): string {
  return breakSpan(exampleClock(12, 10), exampleClock(12, 40));
}

export type WorkDay = Schema<"WorkDayBody">;

export { endOf, startOf } from "./work-day-state.ts";

/** The work days of a week by their date. */
export function daysByDate(
  days: readonly WorkDay[] | undefined,
): Readonly<Record<string, WorkDay>> {
  return Object.fromEntries((days ?? []).map((day) => [day.date, day]));
}

/** The zone the clock of a day is read and typed in: the zone it started in, or the person's. */
export function zoneOfDay(day: WorkDay | undefined, region: Region): string {
  return day?.zone ?? zoneOfNewDay(region.timeZone);
}

/** A moment as the person's clock shows it in the zone of the day (`8:05` or `8:05 AM`). */
export function clockText(moment: string | null | undefined, zone: string, region: Region): string {
  return moment === null || moment === undefined
    ? ""
    : formatTime(moment, { ...region, timeZone: zone });
}

/**
 * A length as the grid writes it (`1:30`); nothing for none. Less than nothing (more booked than
 * worked) is the length with a minus sign, because the writer of lengths knows no negative one.
 */
export function gridLength(wording: Wording, minutes: number | undefined): string {
  if (minutes === undefined) {
    return "";
  }
  return minutes < 0
    ? t("time.sheet.negative-length", { length: wording.lengthOnClock(-minutes) })
    : wording.lengthOnClock(minutes);
}

/** The two figures of a day or of the week against the time booked on issues. */
export interface Figures {
  readonly booked: number;
  readonly notBooked: number;
}

/** A length of the comparison; nothing for a day with nothing worked and nothing booked, or a week of such days. */
export function idleLength(
  wording: Wording,
  minutes: number | undefined,
  figures: Readonly<Figures> | undefined,
): string {
  const idle = figures === undefined || (figures.booked === 0 && figures.notBooked === 0);
  return idle ? "" : gridLength(wording, minutes);
}

/** A typed time of day: its minutes after midnight, or the sentence of what to type instead. */
export function readTimeOfDay(typed: string): CellRead<number> {
  if (typed.trim() === "") {
    return { empty: true };
  }
  const minutes = readClock(typed);
  return minutes === undefined
    ? {
        mistake: t("time.sheet.not-a-time", {
          morning: exampleClock(EXAMPLE_MORNING.hour, EXAMPLE_MORNING.minute),
          afternoon: exampleClock(EXAMPLE_AFTERNOON.hour, EXAMPLE_AFTERNOON.minute),
        }),
      }
    : { value: minutes };
}

function notALength(mistake: Said | null): string {
  return mistake === null
    ? t("time.length.not-a-length", { decimal: formatNumber(EXAMPLE_HOURS) })
    : renderSaid(mistake);
}

/** The dash a break is written with, and read with; the reader also takes an en dash typed by hand. */
const BREAK_DASH = "-";

/** `12:10-12:40` and `10:00-10:15, 12:10-12:40`: the breaks as times, a comma between breaks. */
const BREAK = new RegExp(String.raw`^(.+?)\s*[${BREAK_DASH}\u2013]\s*(.+)$`, "u");

/** A break as every place writes it (the window of the breaks, the history of a day): `12:10-12:40`. */
export function breakSpan(from: string, to: string): string {
  return `${from}${BREAK_DASH}${to}`;
}

/**
 * Typed breaks: the times of each break, never a total, because a time nobody gave is not stored
 * (ADR-0035). A plain total gets the sentence that says to type the times.
 */
export function readBreaks(typed: string): CellRead<BreakTimes> {
  return readBreakLines(typed.split(","));
}

/**
 * The same for breaks that are already one to a line (the window of the breaks), so nothing that
 * joins them for the screen can come between a line and its reading.
 */
export function readBreakLines(lines: readonly string[]): CellRead<BreakTimes> {
  const parts = lines.map((part) => part.trim()).filter((part) => part !== "");
  if (parts.length === 0) {
    return { empty: true };
  }
  if (parts.every((part) => !BREAK.test(part))) {
    return { mistake: t("time.sheet.breaks-as-times", { example: exampleBreak() }) };
  }
  const times = parts.flatMap((part) => {
    const found = breakOf(part);
    return found === undefined ? [] : [found];
  });
  return times.length === parts.length
    ? { value: times }
    : { mistake: t("time.sheet.not-breaks", { example: exampleBreak() }) };
}

/** One break typed as `12:10-12:40`: its two times, or nothing when it is not two times. */
function breakOf(part: string): readonly [number, number] | undefined {
  const found = BREAK.exec(part);
  const from = readClock(found?.[1] ?? "");
  const to = readClock(found?.[2] ?? "");
  return from !== undefined && to !== undefined ? [from, to] : undefined;
}

/** The reader of what is typed in a cell of a work-day row: times of a day, or times of breaks. */
export function readerOf(field: WorkDayField): (typed: string) => CellRead<number | BreakTimes> {
  return field === "breaks" ? readBreaks : readTimeOfDay;
}

/** A typed length of time on an issue: at least a minute, at most a day, as the server takes it. */
export function readBooked(wording: Wording, typed: string): CellRead<number> {
  if (typed.trim() === "") {
    return { empty: true };
  }
  const read = wording.readTimeEntry(lengthTyped(typed));
  return read.minutes === null ? { mistake: notALength(read.mistake) } : { value: read.minutes };
}

/** The sentences of the warnings of a day, in the order of the server. */
export function warningSentences(day: WorkDay | undefined): readonly string[] {
  return (day?.warnings ?? []).flatMap((warning) =>
    isSaid(warning.words) ? [renderSaid(warning.words)] : [],
  );
}

/** The sentence of a warning with the date of the day it is about. */
export interface WarnedDay {
  readonly date: string;
  readonly sentence: string;
}

/** The sentences of the warnings of every day of `days`, in the order of the days. */
export function warningsOfDays(
  days: readonly string[],
  byDate: Readonly<Record<string, WorkDay>>,
): readonly WarnedDay[] {
  return days.flatMap((date) =>
    warningSentences(byDate[date]).map((sentence) => ({ date, sentence })),
  );
}

/** Whether a warning of the given rule stands on the day. */
export function breaksRule(day: WorkDay | undefined, rule: string): boolean {
  return (day?.warnings ?? []).some((warning) => warning.warning["rule"] === rule);
}

/** What a cell is called for people who cannot see its row and day: *Start, Thursday Oct 1, 2026*. */
export function cellLabel(region: Region, row: string, date: string): string {
  return t("time.sheet.cell-label", {
    row,
    day: weekdayAndDate(date, "long", region),
  });
}

/** The key of a cell: it changes when the saved value does, so the cell shows what was saved. */
export function cellKey(...parts: readonly (string | number)[]): string {
  return parts.join("|");
}

/** The dates of the days that were changed with a reason, as the server says. */
export function changedDates(days: readonly WorkDay[] | undefined): readonly string[] {
  return (days ?? []).flatMap((day) => (day.changed === true ? [day.date] : []));
}
