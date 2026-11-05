import { minutesPerHour } from "@bugaboo/api-client";

import { clockPartsOf, dayNumbersOf, isCalendarDay, twoDigits } from "./format.ts";
import { currentRegion, type Region } from "./region.ts";

const NOON = 12;
const HOURS_A_DAY = 24;

/** The clock of the region at a moment (milliseconds since 1970), read as if it were the clock of UTC. */
function wallClockAsUtc(moment: number, region: Region): number {
  const clock = clockPartsOf(moment, region);
  return Date.UTC(clock.year, clock.month - 1, clock.day, clock.hour, clock.minute, clock.second);
}

/**
 * The moment (milliseconds since 1970) at which the clock of the region shows this time on a day the
 * date field wrote (`2027-03-01`); null when the day is none. The offset of the zone is worked out at
 * the moment itself, so a day on which the clocks change still has its hours.
 */
export function momentOnDay(
  day: string,
  time: { readonly hour: number; readonly minute: number; readonly second?: number },
  region: Region = currentRegion(),
): number | null {
  if (!isCalendarDay(day)) {
    return null;
  }
  const [year, month, date] = dayNumbersOf(day);
  const asIfUtc = Date.UTC(year, month - 1, date, time.hour, time.minute, time.second ?? 0);
  const guess = asIfUtc - (wallClockAsUtc(asIfUtc, region) - asIfUtc);
  return asIfUtc - (wallClockAsUtc(guess, region) - guess);
}

/** The time of day of a moment in the region, as a box writes it for the person to change: `08:02`. */
export function timeBoxText(moment: number, region: Region = currentRegion()): string {
  const clock = clockPartsOf(moment, region);
  return `${twoDigits(clock.hour)}:${twoDigits(clock.minute)}`;
}

/** A time of day somebody typed. */
export interface TimeOfDay {
  readonly hour: number;
  readonly minute: number;
}

const TYPED = /^(\d{1,2})(?:[:.h]?(\d{2}))?(?:\s*([ap])\.?m\.?)?$/iu;

/**
 * Reads a time of day as people type it: `8:02`, `08:02`, `8.02`, `802`, `16h30`, `8 pm`, `8:02 AM`.
 * Nothing for text that is no time of day (an hour past 23, minutes past 59, 12 hours with a wrong
 * hour). The words around it are the screen's.
 */
export function readTimeOfDay(text: string): TimeOfDay | undefined {
  const found = TYPED.exec(text.trim());
  if (found === null) {
    return undefined;
  }
  const [, hours = "", minutes = "0", meridiem] = found;
  const minute = Number(minutes);
  let hour = Number(hours);
  if (meridiem !== undefined) {
    if (hour < 1 || hour > NOON) {
      return undefined;
    }
    hour = (hour % NOON) + (meridiem.toLowerCase() === "p" ? NOON : 0);
  }
  return hour < HOURS_A_DAY && minute < minutesPerHour ? { hour, minute } : undefined;
}
