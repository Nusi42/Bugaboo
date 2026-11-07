import { weekdayAndDate } from "../../shared/clock.ts";
import { useRegion } from "../../shared/region.ts";

/** A day as the title of a window says it: its weekday and date, the same on every size of screen. */
export function useDayTitle(date: string | undefined): string {
  const region = useRegion();
  return date === undefined ? "" : weekdayAndDate(date, region);
}
