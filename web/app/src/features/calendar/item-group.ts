import type { CalendarEntry } from "./calendar-data.ts";

/** The group of the status of an issue, for its icon and its tint (R4). */
export function groupOf(entry: CalendarEntry) {
  return entry.issue.status?.status_group ?? "not-started";
}
