import type { Schema } from "@bugaboo/api-client";

import { calendarDayOf, zoneOf } from "../../../shared/format.ts";
import { currentRegion, type Region } from "../../../shared/region.ts";

/** What the day of the key needs of the rules of the organization. */
type Days = Pick<Schema<"AccessKeyRulesBody">, "default_days" | "longest_days">;

const HOURS_BEFORE_MIDNIGHT = 23;
const LAST_MINUTE = 59;
const MS_PER_SECOND = 1000;
const DAY_TEXT = /^(\d{4})-(\d{2})-(\d{2})$/u;

// Every day here is the calendar day of the person's region (its time zone, or the device's when the
// region says `device`), written as the date field writes it (`2027-03-01`): the box, the end of the
// day that is sent and the day the list shows afterwards are all this one day.

/** The day `days` after a day (`2027-02-28` and 1 give `2027-03-01`). */
function addDays(day: string, days: number): string {
  const [year = 0, month = 1, date = 1] = day.split("-").map(Number);
  return new Date(Date.UTC(year, month - 1, date + days)).toISOString().slice(0, day.length);
}

/** Today, the day `days` from now. */
function inDays(days: number, region: Region): string {
  return addDays(calendarDayOf(Date.now(), region), days);
}

/** The clock of the region at an instant, read as if it were the clock of UTC. */
function wallClockAsUtc(instant: number, region: Region): number {
  const parts = new Intl.DateTimeFormat("en-US", {
    ...zoneOf(region),
    hourCycle: "h23",
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "numeric",
    minute: "numeric",
    second: "numeric",
  }).formatToParts(instant);
  const values = new Map<string, number>();
  for (const found of parts) {
    values.set(found.type, Number(found.value));
  }
  const number = (type: string): number => values.get(type) ?? 0;
  return Date.UTC(
    number("year"),
    number("month") - 1,
    number("day"),
    number("hour"),
    number("minute"),
    number("second"),
  );
}

/**
 * The end of a day the date field wrote, in the zone of the region, as an instant in the words of the
 * API; null when the text is no day. The zone's offset is worked out at the instant itself, so a day
 * on which the clocks change still ends at its last second.
 */
export function endOfDay(day: string, region: Region = currentRegion()): string | null {
  const found = DAY_TEXT.exec(day);
  if (found === null) {
    return null;
  }
  const [year = 0, month = 1, date = 1] = found.slice(1).map(Number);
  const asIfUtc = Date.UTC(year, month - 1, date, HOURS_BEFORE_MIDNIGHT, LAST_MINUTE, LAST_MINUTE);
  if (new Date(asIfUtc).toISOString().slice(0, day.length) !== day) {
    return null;
  }
  const guess = asIfUtc - (wallClockAsUtc(asIfUtc, region) - asIfUtc);
  const instant = asIfUtc - (wallClockAsUtc(guess, region) - guess);
  return new Date(Math.trunc(instant / MS_PER_SECOND) * MS_PER_SECOND).toISOString();
}

/** The latest day a key may run out: at least tomorrow, and a day short of the longest time. */
function latestAt(rules: Days): number {
  return Math.max(rules.longest_days - 1, 1);
}

/** The latest day, as the date field writes it. */
export function latestDay(rules: Days, region: Region = currentRegion()): string {
  return inDays(latestAt(rules), region);
}

/** The earliest day, tomorrow, as the date field writes it. */
export function earliestDay(region: Region = currentRegion()): string {
  return inDays(1, region);
}

/** The day filled in before the person picks one: the organization's default, never later than the latest day. */
export function presetDay(rules: Days, region: Region = currentRegion()): string {
  return inDays(Math.min(Math.max(rules.default_days, 1), latestAt(rules)), region);
}
