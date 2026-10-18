import { msPerSecond, secondsPerDay, secondsPerHour, secondsPerMinute } from "@bugaboo/api-client";
import { locale } from "@bugaboo/messages";

import { currentRegion, type NumberFormat, type Region } from "./region.ts";

/** The time zone option of `Intl`: left out for `device`, which is the zone of this device. */
function zoneOf(region: Region): { readonly timeZone?: string } {
  return region.timeZone === "device" ? {} : { timeZone: region.timeZone };
}

const dateStyles = {
  dateTime: { dateStyle: "medium", timeStyle: "short" },
  date: { dateStyle: "long" },
} as const;

/** A space that does not break the line, and a hyphen that does not (U+00A0, U+2011). */
const NO_BREAK_SPACE = " ";
const NO_BREAK_HYPHEN = "‑";

/**
 * Dates and times never wrap in the middle (UX review of M4, A4): a line breaks before a date,
 * not inside it. The spaces and the hyphens of the text become ones that hold it together.
 */
function unbroken(text: string): string {
  return text.replaceAll(" ", NO_BREAK_SPACE).replaceAll("-", NO_BREAK_HYPHEN);
}

/** The words as they are written for an e-mail or a file, where a space is a space and a hyphen a hyphen. */
export function plainWords(text: string): string {
  return text.replaceAll(NO_BREAK_SPACE, " ").replaceAll(NO_BREAK_HYPHEN, "-");
}

/** A day as numbers in the order the person chose: `31.12.2026`, `12/31/2026` or `2026-12-31`. */
function numeric(moment: number, region: Region): string {
  const parts = new Intl.DateTimeFormat("en-US", {
    ...zoneOf(region),
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(moment);
  const values = new Map<string, string>();
  for (const found of parts) {
    values.set(found.type, found.value);
  }
  const part = (type: string): string => values.get(type) ?? "";
  const [year, month, day] = [part("year"), part("month"), part("day")];
  switch (region.dateFormat) {
    case "day-month-year":
      return `${day}.${month}.${year}`;
    case "month-day-year":
      return `${month}/${day}/${year}`;
    case "year-month-day":
    case "device":
      break;
  }
  return `${year}-${month}-${day}`;
}

/** A moment from the server (ISO 8601 text) as people write it: "Oct 5, 2026, 9:30 AM". */
export function formatDateTime(moment: string, region: Region = currentRegion()): string {
  const date = new Date(moment).getTime();
  if (region.dateFormat === "device") {
    return unbroken(
      new Intl.DateTimeFormat(locale, { ...dateStyles.dateTime, ...zoneOf(region) }).format(date),
    );
  }
  const time = new Intl.DateTimeFormat(locale, {
    ...zoneOf(region),
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
  return unbroken(`${numeric(date, region)}, ${time}`);
}

/** The time of day of a moment from the server, in the person's time zone: "10:42". */
export function formatTime(moment: string, region: Region = currentRegion()): string {
  return unbroken(
    new Intl.DateTimeFormat(locale, {
      ...zoneOf(region),
      hour: "numeric",
      minute: "2-digit",
    }).format(new Date(moment).getTime()),
  );
}

/** A day from the server as people write it: "October 5, 2026". */
export function formatDate(moment: string, region: Region = currentRegion()): string {
  const date = new Date(moment).getTime();
  if (region.dateFormat === "device") {
    return unbroken(
      new Intl.DateTimeFormat(locale, { ...dateStyles.date, ...zoneOf(region) }).format(date),
    );
  }
  return unbroken(numeric(date, region));
}

/** The language tag that writes numbers the way each choice does; the device uses its own. */
const numberLocales: Readonly<Record<NumberFormat, string | undefined>> = {
  device: undefined,
  point: "en-US",
  comma: "de-DE",
  space: "fr-FR",
};

/** A number as people write it: "1,234.5", "1.234,5" or "1 234,5". */
export function formatNumber(value: number, region: Region = currentRegion()): string {
  return new Intl.NumberFormat(numberLocales[region.numberFormat]).format(value);
}

/** A day on the calendar ("2026-10-05") as people write it; the time zone never moves it to another day. */
export function formatCalendarDay(day: string, region: Region = currentRegion()): string {
  return formatDate(`${day}T00:00:00Z`, { ...region, timeZone: "UTC" });
}

/** A day (milliseconds since 1970, a date on the person's own calendar) as people write it: "October 5, 2026". */
export function formatDay(moment: number, region: Region = currentRegion()): string {
  // A day on the person's own calendar, picked on this device: the zone of the region does not
  // move it to another day, the way the person wrote it (date format only).
  const onDevice: Region = { ...region, timeZone: "device" };
  return region.dateFormat === "device"
    ? new Intl.DateTimeFormat(locale, dateStyles.date).format(moment)
    : numeric(moment, onDevice);
}

/** Whole days from `nowMs` (milliseconds since 1970) to `moment`; negative when the moment has passed. */
export function daysUntil(moment: string, nowMs: number): number {
  return Math.floor((new Date(moment).getTime() - nowMs) / (msPerSecond * secondsPerDay));
}

/** A number of seconds split into whole days, hours and minutes. */
export function splitUptime(seconds: number): {
  readonly days: number;
  readonly hours: number;
  readonly minutes: number;
} {
  return {
    days: Math.floor(seconds / secondsPerDay),
    hours: Math.floor((seconds % secondsPerDay) / secondsPerHour),
    minutes: Math.floor((seconds % secondsPerHour) / secondsPerMinute),
  };
}

/** The units a span of time is written in, from the longest to the shortest, with their length in seconds. */
const SPAN_UNITS = [
  { unit: "day", seconds: secondsPerDay },
  { unit: "hour", seconds: secondsPerHour },
  { unit: "minute", seconds: secondsPerMinute },
] as const;

/** The longest whole unit that fits in a span of seconds, with how many of it; seconds count as 0 minutes. */
function spanIn(seconds: number): {
  readonly unit: "day" | "hour" | "minute";
  readonly count: number;
} {
  const unit = SPAN_UNITS.find((candidate) => seconds >= candidate.seconds);
  return unit === undefined
    ? { unit: "minute", count: 0 }
    : { unit: unit.unit, count: Math.floor(seconds / unit.seconds) };
}

/** Whole seconds from `moment` (ISO 8601 text) to `nowMs`; never negative (a clock that is a little behind). */
function secondsSince(moment: string, nowMs: number): number {
  return Math.max(0, Math.floor((nowMs - new Date(moment).getTime()) / msPerSecond));
}

/** How long ago a moment was, as people say it: "2 minutes ago", "yesterday"; "now" for under a minute. */
export function formatAgo(moment: string, nowMs: number): string {
  const { unit, count } = spanIn(secondsSince(moment, nowMs));
  return new Intl.RelativeTimeFormat(locale, { numeric: "auto" }).format(-count, unit);
}

/** How long since a moment, short, for a line like "since 2 min": "2 min", "3 hr", "1 day". */
export function formatSince(moment: string, nowMs: number): string {
  const { unit, count } = spanIn(secondsSince(moment, nowMs));
  return new Intl.NumberFormat(locale, {
    style: "unit",
    unit,
    unitDisplay: "short",
  }).format(count);
}
