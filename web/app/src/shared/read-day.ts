import { isCalendarDay } from "./format.ts";
import { currentRegion, type Region } from "./region.ts";

const TWO_DIGITS = 2;
const YEAR_DIGITS = 4;

/** A day typed with a point, a slash, a hyphen or a space between its parts. */
const TYPED_DAY = /^(\d{1,4})[./\s-]\s*(\d{1,2})[./\s-]\s*(\d{1,4})\.?$/u;

/**
 * A day somebody typed into a day box, as the calendar day (`2026-12-31`), in the order of the
 * person's date format; a year first (`2026-12-31`) is read whatever the format is. `undefined`
 * when it reads as none, also for a day the calendar does not have.
 */
export function readDay(text: string, region: Region = currentRegion()): string | undefined {
  const found = TYPED_DAY.exec(text.trim());
  if (found === null) {
    return undefined;
  }
  const [first = "", second = "", third = ""] = found.slice(1);
  const yearFirst = first.length === YEAR_DIGITS || region.dateFormat === "year-month-day";
  const [year, month, date] =
    yearFirst || region.dateFormat === "device"
      ? [first, second, third]
      : region.dateFormat === "day-month-year"
        ? [third, second, first]
        : [third, first, second];
  const day = `${year.padStart(YEAR_DIGITS, "0")}-${month.padStart(TWO_DIGITS, "0")}-${date.padStart(TWO_DIGITS, "0")}`;
  return year.length <= YEAR_DIGITS && isCalendarDay(day) ? day : undefined;
}
