import { dataOf } from "@bugaboo/api-client";
import { setTimeout as sleep } from "node:timers/promises";

import type { ApiSession } from "./api.ts";

/** Today and the week before, as the days of the person are read for the part of the work day. */
const WEEK = 7;
const DAY_MS = 86_400_000;

/** A little over a second: the server takes one change of the work day a second from a person. */
const CHANGE_PAUSE_MS = 1100;

/** A day as a date field writes it (`2027-01-14`), `daysBack` days before now in UTC. */
function dayBack(daysBack: number): string {
  return new Date(Date.now() - daysBack * DAY_MS).toISOString().slice(0, "2027-01-14".length);
}

/** The work days that run, as the server shows them (at most one). */
async function runningDays(session: ApiSession) {
  const days = dataOf(
    await session.client.GET("/api/v1/me/work-days", {
      params: { query: { from: dayBack(WEEK), to: dayBack(-1) } },
    }),
  );
  const running = [];
  for (const day of days.days) {
    if (day.runs) {
      running.push(day);
    }
  }
  return running;
}

/** Whether a work day of the person runs now. */
export async function workDayRuns(session: ApiSession): Promise<boolean> {
  return (await runningDays(session)).length > 0;
}

/** Ends the person's work day if one runs, so a test starts with none running. */
export async function endWorkDayIfRunning(session: ApiSession): Promise<void> {
  const [day] = await runningDays(session);
  if (day !== undefined) {
    await sleep(CHANGE_PAUSE_MS);
    dataOf(
      await session.client.POST("/api/v1/me/work-days/end", { body: { version: day.version } }),
    );
  }
}

/** The date of a day `daysBack` days ago, as the days of the week are named. */
export function dateDaysAgo(daysBack: number): string {
  return dayBack(daysBack);
}

/** Leaves a work day of an earlier date open, as a person who forgot to end it did. */
export async function leaveDayOpen(session: ApiSession, date: string): Promise<void> {
  dataOf(
    await session.client.PATCH("/api/v1/me/work-days/{date}", {
      params: { path: { date } },
      body: {
        version: 0,
        periods: [{ start: `${date}T12:00:00Z`, end: null }],
        breaks: [],
        reason: "Test set-up",
      },
    }),
  );
}
