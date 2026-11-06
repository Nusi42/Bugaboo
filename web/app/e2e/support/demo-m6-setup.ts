import { type ApiSession } from "./api.ts";
import { days } from "./demo-m6-common.ts";
import { changeWorkDay } from "./timesheet.ts";

// What the exit demo of M6 (`m6-exit-demo.spec.ts`) sets up before a step, through the API.

/** How long ago the work day of Ana started: before the timer, so the time booked lies inside it. */
const DAY_MINUTES_AGO = 60;

/**
 * The day of Ana started `DAY_MINUTES_AGO` minutes ago, not at the tap: the timer is moved back by
 * its 25 minutes, and the time it books must lie inside the work day.
 */
export async function startedTheDayEarlier(session: ApiSession): Promise<void> {
  const start = new Date(Date.now() - DAY_MINUTES_AGO * 60_000).toISOString();
  await changeWorkDay(session, days().today, [{ start, end: null }]);
}
