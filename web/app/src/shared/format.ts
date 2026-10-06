import { locale } from "@bugaboo/messages";

import { currentRegion, type NumberFormat, type Region } from "./region.ts";

const SECONDS_PER_DAY = 86_400;
const SECONDS_PER_HOUR = 3600;
const SECONDS_PER_MINUTE = 60;
const MS_PER_SECOND = 1000;

/** The time zone option of `Intl`: left out for `device`, which is the zone of this device. */
function zoneOf(region: Region): { readonly timeZone?: string } {
  return region.timeZone === "device" ? {} : { timeZone: region.timeZone };
}

const dateStyles = {
  dateTime: { dateStyle: "medium", timeStyle: "short" },
  date: { dateStyle: "long" },
} as const;

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
    return new Intl.DateTimeFormat(locale, { ...dateStyles.dateTime, ...zoneOf(region) }).format(
      date,
    );
  }
  const time = new Intl.DateTimeFormat(locale, {
    ...zoneOf(region),
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
  return `${numeric(date, region)}, ${time}`;
}

/** A day from the server as people write it: "October 5, 2026". */
export function formatDate(moment: string, region: Region = currentRegion()): string {
  const date = new Date(moment).getTime();
  if (region.dateFormat === "device") {
    return new Intl.DateTimeFormat(locale, { ...dateStyles.date, ...zoneOf(region) }).format(date);
  }
  return numeric(date, region);
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
  return Math.floor((new Date(moment).getTime() - nowMs) / (MS_PER_SECOND * SECONDS_PER_DAY));
}

/** A number of seconds split into whole days, hours and minutes. */
export function splitUptime(seconds: number): {
  readonly days: number;
  readonly hours: number;
  readonly minutes: number;
} {
  return {
    days: Math.floor(seconds / SECONDS_PER_DAY),
    hours: Math.floor((seconds % SECONDS_PER_DAY) / SECONDS_PER_HOUR),
    minutes: Math.floor((seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE),
  };
}
