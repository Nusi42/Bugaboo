import type { Schema } from "@bugaboo/api-client";

/** What the day of the key needs of the rules of the organization. */
type Days = Pick<Schema<"AccessKeyRulesBody">, "default_days" | "longest_days">;

const DIGITS_OF_YEAR = 4;
const DIGITS_OF_PART = 2;
const HOURS_BEFORE_MIDNIGHT = 23;
const LAST_MINUTE = 59;

/** The day `days` after today (as milliseconds since 1970), by the calendar of the person's own clock, not by UTC. */
export function localDay(days: number): number {
  const today = new Date();
  return new Date(today.getFullYear(), today.getMonth(), today.getDate() + days).getTime();
}

/** A day as the date field writes it (`2027-03-01`). */
export function dayText(moment: number): string {
  const day = new Date(moment);
  const year = String(day.getFullYear()).padStart(DIGITS_OF_YEAR, "0");
  const month = String(day.getMonth() + 1).padStart(DIGITS_OF_PART, "0");
  const date = String(day.getDate()).padStart(DIGITS_OF_PART, "0");
  return `${year}-${month}-${date}`;
}

/**
 * The end of a day the date field wrote, on the person's own clock (as an instant, in the words of the
 * API). The day is the one the person sees in the box and in the list of keys afterwards, whatever the
 * day is in UTC.
 */
export function endOfDay(day: string): string {
  const [year = 0, month = 1, date = 1] = day.split("-").map(Number);
  return new Date(
    year,
    month - 1,
    date,
    HOURS_BEFORE_MIDNIGHT,
    LAST_MINUTE,
    LAST_MINUTE,
  ).toISOString();
}

/** The latest day a key may run out: at least tomorrow, and a day short of the longest time. */
export function latestAt(rules: Days): number {
  return Math.max(rules.longest_days - 1, 1);
}

/** The latest day, as the date field writes it. */
export function latestDay(rules: Days): string {
  return dayText(localDay(latestAt(rules)));
}

/** The earliest day, tomorrow, as the date field writes it. */
export function earliestDay(): string {
  return dayText(localDay(1));
}

/** The day filled in before the person picks one: the organization's default, never later than the latest day. */
export function presetDay(rules: Days): string {
  return dayText(localDay(Math.min(Math.max(rules.default_days, 1), latestAt(rules))));
}
