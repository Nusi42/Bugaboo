import { msPerSecond, secondsPerDay } from "@bugaboo/api-client";
import type { Schema } from "@bugaboo/api-client";

import {
  addDays,
  calendarDayOf,
  clockPartsOf,
  dayNumbersOf,
  isCalendarDay,
} from "../../../shared/format.ts";
import { currentRegion, type Region } from "../../../shared/region.ts";

/** What the day of the key needs of the rules of the organization. */
type Days = Pick<Schema<"AccessKeyRulesBody">, "default_days" | "longest_days">;

const HOURS_BEFORE_MIDNIGHT = 23;
const LAST_MINUTE = 59;
const MS_PER_DAY = msPerSecond * secondsPerDay;

// Every day here is the calendar day of the person's region (its time zone, or the device's when the
// region says `device`) of a moment, written as the date field writes it (`2027-03-01`). The server
// ends a key at a moment, now plus a number of days, so the day in the box, the latest day and the
// day the list shows afterwards are the day of that moment: one rule, the server's moments.

/** The day of the moment `days` from now, on the calendar of the region. */
function dayOfMomentIn(days: number, region: Region): string {
  return calendarDayOf(Date.now() + days * MS_PER_DAY, region);
}

/** The later of two days (the date field writes days so that text order is calendar order). */
function later(a: string, b: string): string {
  return a >= b ? a : b;
}

/** The clock of the region at a moment, read as if it were the clock of UTC. */
function wallClockAsUtc(moment: number, region: Region): number {
  const clock = clockPartsOf(moment, region);
  return Date.UTC(clock.year, clock.month - 1, clock.day, clock.hour, clock.minute, clock.second);
}

/**
 * The end of a day the date field wrote, in the zone of the region, as milliseconds since 1970; null
 * when the text is no day. The zone's offset is worked out at the moment itself, so a day on which
 * the clocks change still ends at its last second.
 */
function endOfDayAt(day: string, region: Region): number | null {
  if (!isCalendarDay(day)) {
    return null;
  }
  const [year, month, date] = dayNumbersOf(day);
  const asIfUtc = Date.UTC(year, month - 1, date, HOURS_BEFORE_MIDNIGHT, LAST_MINUTE, LAST_MINUTE);
  const guess = asIfUtc - (wallClockAsUtc(asIfUtc, region) - asIfUtc);
  return asIfUtc - (wallClockAsUtc(guess, region) - guess);
}

/** The end of a day, as a moment in the words of the API; null when the text is no day. */
export function endOfDay(day: string, region: Region = currentRegion()): string | null {
  const end = endOfDayAt(day, region);
  return end === null ? null : new Date(Math.trunc(end / msPerSecond) * msPerSecond).toISOString();
}

/**
 * The moment a key picked for a day ends: the end of that day in the region, or the longest time
 * from now when that comes first (the server's limit); null when the text is no day.
 */
export function endOfKey(
  day: string,
  rules: Days,
  region: Region = currentRegion(),
): string | null {
  const end = endOfDayAt(day, region);
  if (end === null) {
    return null;
  }
  const longest = Date.now() + rules.longest_days * MS_PER_DAY;
  return new Date(Math.trunc(Math.min(end, longest) / msPerSecond) * msPerSecond).toISOString();
}

/** What a key is made with for the day in the form: `null` until a day is picked (the default of the server); a day that is no day is not valid. */
export function keyEndOf(
  day: string | null,
  rules: Days,
  region: Region = currentRegion(),
): { readonly valid: boolean; readonly expiresAt: string | null } {
  if (day === null) {
    return { valid: true, expiresAt: null };
  }
  const expiresAt = endOfKey(day, rules, region);
  return { valid: expiresAt !== null, expiresAt };
}

/** The earliest day, tomorrow, as the date field writes it. */
export function earliestDay(region: Region = currentRegion()): string {
  return addDays(calendarDayOf(Date.now(), region), 1);
}

/** The latest day: the day of the moment the longest time from now ends on, never before tomorrow. */
export function latestDay(rules: Days, region: Region = currentRegion()): string {
  return later(dayOfMomentIn(rules.longest_days, region), earliestDay(region));
}

/** The day filled in before the person picks one: the day of the default, never later than the latest day. */
export function presetDay(rules: Days, region: Region = currentRegion()): string {
  const days = Math.min(Math.max(rules.default_days, 1), rules.longest_days);
  return later(dayOfMomentIn(days, region), earliestDay(region));
}
