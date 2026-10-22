/**
 * A value of an own field as people read it, where the screen words it itself (CF-01): a length of
 * time is seconds in the API and "1h 30m" on the screen. The wording of a field on the issue page.
 * A change in the history and in a notification is worded by `bb-core` (`wording`), which the
 * vectors in `tools/testdata/value-words.json` hold to this code (ADR-0033).
 */

import { secondsPerHour, secondsPerMinute, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { formatCalendarDay, formatDateTime, formatNumber } from "./format.ts";
import { currentRegion, type Region } from "./region.ts";

/** Seconds as "2h 5m", "45m" or "0m"; whole minutes (a duration is typed in minutes at the least). */
export function durationText(seconds: number): string {
  const hours = Math.floor(seconds / secondsPerHour);
  const minutes = Math.floor((seconds % secondsPerHour) / secondsPerMinute);
  const [hour, minute] = [t("fields.unit-hour"), t("fields.unit-minute")];
  if (hours === 0) {
    return `${minutes}${minute}`;
  }
  return minutes === 0 ? `${hours}${hour}` : `${hours}${hour} ${minutes}${minute}`;
}

function textOf(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function numberTextOf(value: unknown, region: Region): string | undefined {
  return typeof value === "number" ? formatNumber(value, region) : textOf(value);
}

const DAY = /^\d{4}-\d{2}-\d{2}$/u;

/** A day that exists on the calendar: "2026-02-30" does not, and the browser would roll it over. */
function isCalendarDay(value: string): boolean {
  const moment = new Date(`${value}T00:00:00Z`);
  return (
    DAY.test(value) && !Number.isNaN(moment.getTime()) && moment.toISOString().startsWith(value)
  );
}

/** A moment from the server; text that is no moment has no words (the browser would throw). */
function momentTextOf(value: unknown, region: Region): string | undefined {
  return typeof value === "string" && !Number.isNaN(new Date(value).getTime())
    ? formatDateTime(value, region)
    : undefined;
}

/** How a value of each kind is worded; the kinds that are only words are the text itself. */
const worded: Readonly<
  Record<Schema<"FieldType">, (value: unknown, region: Region) => string | undefined>
> = {
  "yes-no": (value) =>
    typeof value === "boolean" ? (value ? t("fields.yes") : t("fields.no")) : undefined,
  duration: (value) => (typeof value === "number" && value >= 0 ? durationText(value) : undefined),
  "whole-number": numberTextOf,
  decimal: numberTextOf,
  date: (value, region) =>
    typeof value === "string" && isCalendarDay(value)
      ? formatCalendarDay(value, region)
      : undefined,
  "date-time": momentTextOf,
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

/** The kinds whose words depend on what the reader chose under *Region*. */
const regional: ReadonlySet<Schema<"FieldType">> = new Set([
  "whole-number",
  "decimal",
  "date",
  "date-time",
]);

/**
 * The words of a number, a day or a moment in the region of the reader, which the shared wording
 * takes as they are (the browser writes them with `Intl`, whatever the reader chose, `device`
 * included); `undefined` for any other kind and for a value that does not fit its kind.
 */
export function regionWords(
  kind: Schema<"FieldType">,
  value: unknown,
  region: Region = currentRegion(),
): string | undefined {
  return regional.has(kind) ? worded[kind](value, region) : undefined;
}

/**
 * A value of an own field that the screen words itself, as people read it (yes or no, a length of
 * time, a number, a day, a moment); `undefined` for a value that does not fit the kind.
 */
export function ownValueText(
  kind: Schema<"FieldType">,
  value: unknown,
  region: Region = currentRegion(),
): string | undefined {
  return worded[kind](value, region);
}
