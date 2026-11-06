import { queryClient } from "./query-client.ts";
import { queryKeys, reread, refreshRoutes, routeStartsWith } from "./query-keys.ts";

/**
 * The keys of what the screens of time keep in the query cache. They are written here, next to the
 * keys of the search (`search-keys.ts`), because `query-keys.ts` is as long as a file may be.
 */
export const timeKeys = {
  /** The person's timer, with how far the server's clock was from the browser's when it was read. */
  timer: () => ["time", "timer"],
  /** What a new time entry on an issue starts with. */
  defaults: (issue: string) => ["time", "defaults", issue],
  /** The person's work days from one date to another (both included). */
  workDays: (first: string, last: string) => ["time", "work-days", first, last],
  /** Everything of time that is read from the server: what a change of the timer or the work day renews. */
  all: () => ["time"],
  /** A week of the person's own time entries, by its first and last date. */
  entries: (from: string, to: string) => ["time", "entries", from, to],
  /** The people to choose from for *Working hours of others*, by the words searched. */
  people: (search: string) => ["time", "people", search],
  /** The history of a work day of a person ('' for one's own) by its date. */
  history: (person: string, date: string) => ["time", "history", person, date],
} as const;

/** The routes the timesheet reads: one's own work days and those of one person at a time. */
const TIMESHEET_ROUTES = ["/api/v1/me/work-days", "/api/v1/people/{person}/work-days"];

/**
 * A time entry or a work day was booked, changed or taken back: the timesheet and the comparison
 * of its footer are read again, and so are the totals of the issues (the time spent).
 */
export function refreshTimesheet(): void {
  reread({ queryKey: timeKeys.all() });
  refreshRoutes(TIMESHEET_ROUTES);
  reread({ queryKey: queryKeys.issues.time() });
}

/**
 * Reads the week again and answers when it is read, so the next change of a day is made on the
 * version this one left (`refreshTimesheet` answers at once).
 */
export async function rereadTimesheet(): Promise<void> {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: timeKeys.all() }),
    queryClient.invalidateQueries({ predicate: routeStartsWith(TIMESHEET_ROUTES) }),
  ]);
}
