import type { CalendarEntry } from "./calendar-data.ts";

/** The entries by the day they stand on now: where the server has them, or where a move on its way put them. */
export function entriesByDay(
  entries: readonly CalendarEntry[],
  dayOf: (entry: CalendarEntry) => string,
): ReadonlyMap<string, readonly CalendarEntry[]> {
  const days = new Map<string, CalendarEntry[]>();
  for (const entry of entries) {
    const day = dayOf(entry);
    days.set(day, [...(days.get(day) ?? []), entry]);
  }
  return days;
}
