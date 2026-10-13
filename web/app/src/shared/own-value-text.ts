/**
 * A value of an own field as people read it, where the screen words it itself (CF-01): a length of
 * time is seconds in the API and "1h 30m" on the screen. The one wording, for the field on the
 * issue page and for a change in the history.
 */

import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { formatDate, formatDateTime, formatNumber } from "./format.ts";

export const SECONDS_PER_MINUTE = 60;
const MINUTES_PER_HOUR = 60;
export const SECONDS_PER_HOUR = SECONDS_PER_MINUTE * MINUTES_PER_HOUR;

/** Seconds as "2h 5m", "45m" or "0m"; whole minutes (a duration is typed in minutes at the least). */
export function durationText(seconds: number): string {
  const hours = Math.floor(seconds / SECONDS_PER_HOUR);
  const minutes = Math.floor((seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE);
  const [hour, minute] = [t("fields.unit-hour"), t("fields.unit-minute")];
  if (hours === 0) {
    return `${minutes}${minute}`;
  }
  return minutes === 0 ? `${hours}${hour}` : `${hours}${hour} ${minutes}${minute}`;
}

function textOf(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function numberTextOf(value: unknown): string | undefined {
  return typeof value === "number" ? formatNumber(value) : textOf(value);
}

/** How a value of each kind is worded; the kinds that are only words are the text itself. */
const worded: Readonly<Record<Schema<"FieldType">, (value: unknown) => string | undefined>> = {
  "yes-no": (value) =>
    typeof value === "boolean" ? (value ? t("fields.yes") : t("fields.no")) : undefined,
  duration: (value) => (typeof value === "number" ? durationText(value) : undefined),
  "whole-number": numberTextOf,
  decimal: numberTextOf,
  date: (value) => (typeof value === "string" ? formatDate(`${value}T00:00:00Z`) : undefined),
  "date-time": (value) => (typeof value === "string" ? formatDateTime(value) : undefined),
  text: textOf,
  "long-text": textOf,
  "web-address": textOf,
  "pick-one": textOf,
  "pick-several": textOf,
  person: textOf,
  people: textOf,
  group: textOf,
  issue: textOf,
};

/**
 * A value of an own field that the screen words itself, as people read it (yes or no, a length of
 * time, a number, a day, a moment); `undefined` for a value that does not fit the kind.
 */
export function ownValueText(kind: Schema<"FieldType">, value: unknown): string | undefined {
  return worded[kind](value);
}
