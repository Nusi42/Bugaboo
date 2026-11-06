import { dataOf, msPerDay } from "@bugaboo/api-client";

import { type ApiSession } from "../../app/e2e/support/api.ts";
import { listPage } from "../../app/e2e/support/perf-reads.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";
import { type Context } from "./scenarios.ts";

/** How many projects the issues to read are taken from, and how many pages of each. */
const PROJECTS_READ = 8;
const PAGES_READ = 3;

/** The readable IDs of the pages of the list of one project, up to `PAGES_READ` pages. */
async function keysOf(
  session: ApiSession,
  project: string,
  cursor?: string,
  page = 0,
): Promise<readonly string[]> {
  const read = await listPage(session, project, cursor);
  const keys: string[] = [];
  for (const item of read.items) {
    if (typeof item.readable_id === "string") {
      keys.push(item.readable_id);
    }
  }
  const next = read.next_cursor ?? undefined;
  if (next === undefined || page + 1 >= PAGES_READ) {
    return keys;
  }
  return [...keys, ...(await keysOf(session, project, next, page + 1))];
}

/** The readable IDs of the first pages of the lists of some projects, spread over all of them. */
export async function someIssues(
  session: ApiSession,
  projects: readonly string[],
): Promise<readonly string[]> {
  const step = Math.max(1, Math.floor(projects.length / PROJECTS_READ));
  const chosen = projects.filter((_, at) => at % step === 0).slice(0, PROJECTS_READ);
  const keys: string[] = [];
  await inOrder(
    chosen.map((project) => async () => {
      keys.push(...(await keysOf(session, project)));
    }),
  );
  return keys;
}

/** The readable IDs of the first `count` issues of the list of a project. */
export async function firstIssues(
  session: ApiSession,
  project: string,
  count: number,
  cursor?: string,
  found: readonly string[] = [],
): Promise<readonly string[]> {
  const page = await listPage(session, project, cursor);
  const keys = [...found];
  for (const item of page.items) {
    if (typeof item.readable_id === "string") {
      keys.push(item.readable_id);
    }
  }
  const next = page.next_cursor ?? undefined;
  return next === undefined || keys.length >= count
    ? keys.slice(0, count)
    : firstIssues(session, project, count, next, keys);
}

/** The date `days` days after `day` (`2027-01-14`), in UTC; the driver is plain Node and cannot load the screens' code. */
function addDays(day: string, days: number): string {
  return new Date(Date.parse(day) + days * msPerDay).toISOString().slice(0, day.length);
}

/** How far back the last work day of the person is looked for, in days (the most one read takes). */
const LOOK_BACK_DAYS = 61;
/** The days of a week. */
const WEEK_DAYS = 7;

/** The last week the person worked: the seven days that end with their newest work day. */
export async function lastWeek(session: ApiSession): Promise<Context["week"]> {
  const today = new Date().toISOString().slice(0, "2027-01-14".length);
  const read = dataOf(
    await session.client.GET("/api/v1/me/work-days", {
      params: { query: { from: addDays(today, -LOOK_BACK_DAYS), to: today } },
    }),
  );
  let last: string | undefined;
  for (const day of read.days) {
    if (last === undefined || day.date > last) {
      last = day.date;
    }
  }
  if (last === undefined) {
    throw new Error("The dataset has no work day of the Administrator to measure the timesheet.");
  }
  return { first: addDays(last, 1 - WEEK_DAYS), last };
}
