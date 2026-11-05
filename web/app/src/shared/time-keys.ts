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
} as const;
