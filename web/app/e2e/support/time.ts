import { dataOf } from "@bugaboo/api-client";
import { setTimeout as sleep } from "node:timers/promises";

import type { ApiSession } from "./api.ts";
import { versionOf, type MadeIssue } from "./issues.ts";

/** The key of the own field a starter field of time entries is (`activity`, `cost-center`), as the timer says. */
export async function starterField(session: ApiSession, mark: string): Promise<string> {
  const timer = dataOf(await session.client.GET("/api/v1/me/timer"));
  const found = timer.starters[mark];
  if (found === undefined) {
    throw new Error(`no starter field ${mark}`);
  }
  return found;
}

/** The ID of the choice of a field with this name, made when the field has none yet. */
export async function choiceNamed(
  session: ApiSession,
  field: string,
  name: string,
): Promise<string> {
  const shown = dataOf(
    await session.client.GET("/api/v1/fields/{field}", { params: { path: { field } } }),
  );
  const found = shown.choices.find((choice) => choice.name === name);
  if (found !== undefined) {
    return found.id;
  }
  const added = dataOf(
    await session.client.POST("/api/v1/fields/{field}/choices", {
      params: { path: { field } },
      body: { version: shown.version, name },
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

/** A little over a second: the server lets a person start or switch the timer once a second. */
const TIMER_PAUSE_MS = 1100;

/** Starts the timer of the person of the session on an issue (after the pause the server asks for). */
export async function startTimerOn(session: ApiSession, issue: MadeIssue): Promise<void> {
  await sleep(TIMER_PAUSE_MS);
  dataOf(await session.client.POST("/api/v1/me/timer", { body: { issue: issue.key } }));
}

/** Moves the start of the person's running timer `minutes` and a few seconds into the past. */
export async function startedMinutesAgo(session: ApiSession, minutes: number): Promise<void> {
  const timer = dataOf(await session.client.GET("/api/v1/me/timer"));
  const SECONDS = 5;
  const started = new Date(Date.now() - (minutes * 60 + SECONDS) * 1000).toISOString();
  dataOf(
    await session.client.POST("/api/v1/me/timer/start-time", {
      body: { version: timer.version, started_at: started },
    }),
  );
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
  /** The values of its own fields, by the key of the field. */
  readonly fields: Readonly<Record<string, unknown>>;
}

/** The entries the person booked on an issue this week, in the order the server lists them. */
export async function entriesBookedOn(
  session: ApiSession,
  issue: MadeIssue,
): Promise<readonly BookedEntry[]> {
  const week = dataOf(
    await session.client.GET("/api/v1/me/time-entries", {
      params: { query: { fields: "issue,minutes,note,own_fields" } },
    }),
  );
  const booked: BookedEntry[] = [];
  for (const entry of week.items) {
    if (entry.issue === issue.id) {
      booked.push({ minutes: entry.minutes, note: entry.note, fields: entry.own_fields ?? {} });
    }
  }
  return booked;
}
