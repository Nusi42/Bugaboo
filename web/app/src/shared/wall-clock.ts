import { minutesPerHour } from "@bugaboo/api-client";

import { momentOfClock, readClock } from "./clock.ts";
import { clockPartsOf, isCalendarDay, timeOfDayText } from "./format.ts";
import { currentRegion, type Region } from "./region.ts";

const MS_PER_SECOND = 1000;

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
  const start = Date.parse(
    momentOfClock(day, time.hour * minutesPerHour + time.minute, region.timeZone),
  );
  return start + (time.second ?? 0) * MS_PER_SECOND;
}

/** The time of day of a moment in the region, as a box writes it for the person to change: `8:02`. */
export function timeBoxText(moment: number, region: Region = currentRegion()): string {
  const clock = clockPartsOf(moment, region);
  return timeOfDayText(clock.hour, clock.minute, region);
}

/** A time of day somebody typed. */
export interface TimeOfDay {
  readonly hour: number;
  readonly minute: number;
}

/**
 * Reads a time of day as people type it (`readClock`, the one reader): `8:02`, `08:02`, `8.02`, `802`,
 * `16h30`, `8 pm`, `8:02 AM`. Nothing for text that is no time of day.
 */
export function readTimeOfDay(text: string): TimeOfDay | undefined {
  const minutes = readClock(text);
  return minutes === undefined
    ? undefined
    : { hour: Math.floor(minutes / minutesPerHour), minute: minutes % minutesPerHour };
}
