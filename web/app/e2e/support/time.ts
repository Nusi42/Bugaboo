import { dataOf, type Schema } from "@bugaboo/api-client";
import { setTimeout as sleep } from "node:timers/promises";

import { signedInAdmin } from "./access.ts";
import type { ApiSession } from "./api.ts";
import { createIssue, versionOf, type MadeIssue } from "./issues.ts";
import { choiceBody } from "./fields.ts";
import type { Instance } from "./server.ts";

/** The key of the own field a starter field of time entries is (`activity`, `cost-center`), as the timer says. */
export async function starterField(session: ApiSession, mark: string): Promise<string> {
  const timer = dataOf(await session.client.GET("/api/v1/me/timer"));
  const found = timer.starters[mark];
  if (found === undefined) {
    throw new Error(`no starter field ${mark}`);
  }
  return found;
}

/** The ID of the choice of a field with this name, made (with the number, if one is given) when the field has none yet. */
export async function choiceNamed(
  session: ApiSession,
  field: string,
  name: string,
  number?: string,
): Promise<string> {
  const shown: Schema<"FieldBody"> = dataOf(
    await session.client.GET("/api/v1/fields/{field}", { params: { path: { field } } }),
  );
  const found = shown.choices.find((choice) => choice.name === name);
  if (found !== undefined) {
    return found.id;
  }
  const added: Schema<"FieldBody"> = dataOf(
    await session.client.POST("/api/v1/fields/{field}/choices", {
      params: { path: { field } },
      body: choiceBody(shown.version, name, number),
    }),
  );
  const made = added.choices.find((choice) => choice.name === name);
  if (made === undefined) {
    throw new Error(`the choice ${name} was not made`);
  }
  return made.id;
}

/** Gives an issue its default of a field of time entries: what every entry booked on it starts with. */
export async function setTimeDefault(
  session: ApiSession,
  issue: MadeIssue,
  field: string,
  choice: string,
): Promise<void> {
  dataOf(
    await session.client.PATCH("/api/v1/issues/{issue}", {
      params: { path: { issue: issue.key } },
      body: { version: await versionOf(session, issue), fields: { [field]: choice } },
    }),
  );
}

/** Gives the issue an Activity (Testing) and a Cost center (4100 Product development) for its time, so the entry of a stop or a booking is filled in. */
export async function giveDefaults(admin: ApiSession, issue: MadeIssue): Promise<void> {
  const activity = await starterField(admin, "activity");
  const costCenter = await starterField(admin, "cost-center");
  await setTimeDefault(admin, issue, activity, await choiceNamed(admin, activity, "Testing"));
  await setTimeDefault(
    admin,
    issue,
    costCenter,
    await choiceNamed(admin, costCenter, "Product development", "4100"),
  );
}

/** A little over a second: the server lets a person start or switch the timer once a second. */
const TIMER_PAUSE_MS = 1100;

/** Starts the timer of the person of the session on an issue (after the pause the server asks for). */
export async function startTimerOn(session: ApiSession, issue: MadeIssue): Promise<void> {
  await sleep(TIMER_PAUSE_MS);
  dataOf(await session.client.POST("/api/v1/me/timer", { body: { issue: issue.key } }));
}

/** A day in seconds. */
const DAY_SECONDS = 86_400;

/**
 * The moment `seconds` ago, but not before one minute past the start of today, both in UTC (the zone
 * of the people of the test server) and on the computer of the test (the zone of the browser): a work
 * day must start on its own date, so a test in the first hour after midnight starts it as early as that
 * date allows. A start more than a day back (a timer that runs long on purpose) is not held.
 */
export function earlierToday(seconds: number): string {
  const now = Date.now();
  if (seconds > DAY_SECONDS) {
    return new Date(now - seconds * 1000).toISOString();
  }
  const local = new Date(now);
  local.setHours(0, 1, 0, 0);
  const utc = new Date(now);
  utc.setUTCHours(0, 1, 0, 0);
  const earliest = Math.max(local.getTime(), utc.getTime());
  return new Date(Math.max(now - seconds * 1000, earliest)).toISOString();
}

/**
 * Moves the start of the person's running timer `minutes` and a few seconds into the past; with
 * `withinToday` not before the start of the day `earlierToday` gives (for a timer whose time must lie in
 * a work day that started on that date).
 */
export async function startedMinutesAgo(
  session: ApiSession,
  minutes: number,
  withinToday = false,
): Promise<void> {
  const timer = dataOf(await session.client.GET("/api/v1/me/timer"));
  const SECONDS = 5;
  const seconds = minutes * 60 + SECONDS;
  const started = withinToday
    ? earlierToday(seconds)
    : new Date(Date.now() - seconds * 1000).toISOString();
  dataOf(
    await session.client.POST("/api/v1/me/timer/start-time", {
      body: { version: timer.version, started_at: started },
    }),
  );
}

/** Whether the server has marked the person's timer as running longer than a work day (its job does, a little after). */
export async function timerRunsLong(session: ApiSession): Promise<boolean> {
  const timer = dataOf(await session.client.GET("/api/v1/me/timer"));
  return timer.running?.runs_long === true;
}

/** The readable ID of the issue the person's timer runs on; none while it stands still. */
export async function timerRunsOn(session: ApiSession): Promise<string | undefined> {
  const timer = dataOf(await session.client.GET("/api/v1/me/timer"));
  return timer.running?.readable_id ?? undefined;
}

/** Throws the person's timer away, so the next test of the file starts with none. */
export async function clearTimer(session: ApiSession): Promise<void> {
  const timer = dataOf(await session.client.GET("/api/v1/me/timer"));
  if (timer.running !== undefined && timer.running !== null) {
    dataOf(
      await session.client.POST("/api/v1/me/timer/throw-away", {
        body: { version: timer.version },
      }),
    );
  }
}

/** What a person's own time entry holds, as the tests look at it. */
export interface BookedEntry {
  readonly minutes: number;
  readonly note: string;
  /** Whether it is marked *Billable*; nothing while the organization does not offer the mark. */
  readonly billable: boolean | undefined;
  /** The values of its own fields, by the key of the field. */
  readonly fields: Readonly<Record<string, unknown>>;
}

/**
 * The entries the person booked on an issue this week (or from one date to another), in the order
 * the server lists them.
 */
export async function entriesBookedOn(
  session: ApiSession,
  issue: MadeIssue,
  days?: { readonly from: string; readonly to: string },
): Promise<readonly BookedEntry[]> {
  const week = dataOf(
    await session.client.GET("/api/v1/me/time-entries", {
      params: { query: { fields: "issue,minutes,note,billable,own_fields", ...days } },
    }),
  );
  const booked: BookedEntry[] = [];
  for (const entry of week.items) {
    if (entry.issue === issue.id) {
      booked.push({
        minutes: entry.minutes,
        note: entry.note,
        billable: entry.billable ?? undefined,
        fields: entry.own_fields ?? {},
      });
    }
  }
  return booked;
}

/** An issue of the project WEB whose time starts with the cost center *4100 Product development*. */
export async function issueStartingWithProductDevelopment(server: Instance): Promise<MadeIssue> {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Printer jams");
  const costCenter = await starterField(admin, "cost-center");
  await setTimeDefault(
    admin,
    issue,
    costCenter,
    await choiceNamed(admin, costCenter, "Product development", "4100"),
  );
  await admin.dispose();
  return issue;
}
