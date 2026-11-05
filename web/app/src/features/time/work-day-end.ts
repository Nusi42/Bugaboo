import { currentRegion } from "../../shared/region.ts";
import { momentOnDay, readTimeOfDay } from "../../shared/wall-clock.ts";
import type { WorkDay } from "./work-day-state.ts";

/**
 * The moment a day that was left open ended, from the time of day somebody typed: on the date of the
 * day, by the clock of the zone the day was recorded in (`day.zone`), not of the zone the person's
 * *Region* names now. Nothing for text that is no time of day.
 */
export function endOfOpenDay(day: Pick<WorkDay, "date" | "zone">, typed: string): number | null {
  const time = readTimeOfDay(typed);
  return time === undefined
    ? null
    : momentOnDay(day.date, time, { ...currentRegion(), timeZone: day.zone });
}
