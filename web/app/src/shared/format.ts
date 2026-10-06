import { locale } from "@bugaboo/messages";

const SECONDS_PER_DAY = 86_400;
const SECONDS_PER_HOUR = 3600;
const SECONDS_PER_MINUTE = 60;
const MS_PER_SECOND = 1000;

const dateTime = new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" });
const date = new Intl.DateTimeFormat(locale, { dateStyle: "long" });

/** A moment from the server (ISO 8601 text) as people write it: "Oct 5, 2026, 9:30 AM". */
export function formatDateTime(moment: string): string {
  return dateTime.format(new Date(moment));
}

/** A day from the server as people write it: "October 5, 2026". */
export function formatDate(moment: string): string {
  return date.format(new Date(moment));
}

/** A day (milliseconds since 1970, a date on the person's own calendar) as people write it: "October 5, 2026". */
export function formatDay(moment: number): string {
  return date.format(moment);
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
