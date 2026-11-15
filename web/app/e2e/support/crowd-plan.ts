import { ApiError, dataOf } from "@bugaboo/api-client";
import { expect } from "@playwright/test";

import { ada, ownId, signInThroughApi, type Person } from "./accounts.ts";
import type { ApiSession } from "./api.ts";
import { crowd, prepareCrowd } from "./crowd.ts";
import { createIssue, issueOf, versionOf } from "./issues.ts";
import { inOrder } from "./sequence.ts";
import type { Instance } from "./server.ts";
import { changeIssue, dayFromToday, relateBlocking } from "./timeline.ts";

// Ten people planning at the same time (architecture §11, M8 WP-12, PLAN-01, PLAN-11, PLAN-13):
// the crowd of the project WEB with the issues they plan. The people move bars, accept a warning,
// enter an away time and move issues after another; the rules of the run are judged by
// `bb-load-check` (crates/bb-load-check/src/plan.rs), what each person sees is checked here.

/** The bars ten people move, two each. */
export const barCount = 5;

/** The issues the scenarios work on, by their use. WEB-1 to WEB-12 are the issues of the crowd. */
export const planIssues = {
  bars: ["WEB-13", "WEB-14", "WEB-15", "WEB-16", "WEB-17"],
  early: "WEB-18",
  late: "WEB-19",
  away: "WEB-20",
  first: "WEB-21",
  middle: "WEB-22",
  after: "WEB-23",
} as const;

/** The person whose away time is entered: the last of the crowd. */
export const awayPerson = crowd.at(-1);

/** The titles in the order the issues are made. */
const titles = [
  ...Array.from({ length: barCount }, (_, at) => `Bar ${String(at + 1)}`),
  "Early work",
  "Late work",
  "Work of the away person",
  "First step",
  "Then step",
  "Step after",
];

/** Gets the crowd ready, with the issues the planning scenarios work on, dated from today. */
export async function preparePlanCrowd(server: Instance): Promise<void> {
  await prepareCrowd(server);
  const admin = await signInThroughApi(server, ada);
  await inOrder(titles.map((title) => () => createIssue(admin, "WEB", title)));
  await inOrder(planIssues.bars.map((key) => () => changeIssue(admin, key, dates(1, 3))));
  // Early work ends on a day Late work has started: a delay, a warning to accept.
  await changeIssue(admin, planIssues.early, dates(1, 4));
  await changeIssue(admin, planIssues.late, dates(3, 5));
  await relateBlocking(admin, planIssues.early, planIssues.late);
  const away = await signInThroughApi(server, awayPerson ?? ada);
  await changeIssue(admin, planIssues.away, {
    ...dates(8, 12),
    assigned_to: [await ownId(away)],
  });
  await away.dispose();
  // First step, then step, step after: a delay of the second that moves the third too.
  await changeIssue(admin, planIssues.first, dates(1, 4));
  await changeIssue(admin, planIssues.middle, dates(3, 5));
  await changeIssue(admin, planIssues.after, dates(6, 6));
  await relateBlocking(admin, planIssues.first, planIssues.middle);
  await relateBlocking(admin, planIssues.middle, planIssues.after);
  await admin.dispose();
}

/** The start and due date, `from` and `to` days after today. */
export function dates(from: number, to: number) {
  return { start_date: dayFromToday(from), due_date: dayFromToday(to) };
}

/** A move that the server kept: the issue, its version after the move and its dates. */
export interface KeptMove {
  readonly key: string;
  readonly version: number;
  readonly start: string;
  readonly due: string;
}

/** What the server answered to a request: `kept`, or the code of its refusal. */
export async function answerOf(request: Readonly<Promise<unknown>>): Promise<string> {
  try {
    await request;
    return "kept";
  } catch (error) {
    if (error instanceof ApiError) {
      return error.problem?.code ?? `http ${String(error.status)}`;
    }
    throw error;
  }
}

/** The code of the refusal of a move that came second. */
export const STALE = "edit.version-changed";

/**
 * Moves the bar of an issue to these days, on the version the person read; a move that came second
 * is sent again on the newer version, as *Use mine* does. Answers the move the server kept and
 * how many sends were refused as stale.
 */
export async function moveBar(
  session: ApiSession,
  key: string,
  to: { readonly start_date: string; readonly due_date: string },
  stale = 0,
): Promise<{ readonly kept: KeptMove; readonly stale: number }> {
  const version = await versionOf(session, await issueOf(session, key));
  const sent = session.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: key } },
    body: { version, ...to },
  });
  const answer = await answerOf(sent);
  if (answer === STALE) {
    return moveBar(session, key, to, stale + 1);
  }
  expect(answer).toBe("kept");
  const issue = dataOf(await sent);
  return {
    kept: { key, version: issue.version ?? 0, start: to.start_date, due: to.due_date },
    stale,
  };
}

/** The page of the timeline of WEB for the days around today, as a person reads it through the API. */
export async function readPlan(session: ApiSession) {
  return dataOf(
    await session.client.POST("/api/v1/timeline", {
      body: {
        search_text: "project is WEB",
        timeline: { start_field: null, end_field: null, zoom: "week" },
        first: dayFromToday(-7),
        last: dayFromToday(60),
      },
    }),
  );
}

/** What the plan says of one warning, as much as these tests need. */
export interface WarningRow {
  readonly issue: string;
  readonly kind: string;
  /** Whether the warning is faded because somebody accepted it. */
  readonly accepted: boolean;
  /** The day *Move it after* starts the issue after, if the warning offers it. */
  readonly moveAfter: string | null;
}

/** The warnings of the plan of WEB, as a person reads them. */
export async function readWarnings(session: ApiSession): Promise<readonly WarningRow[]> {
  const rows: WarningRow[] = [];
  for (const warning of (await readPlan(session)).warnings) {
    rows.push({
      issue: warning.issue,
      kind: warning.kind,
      accepted: warning.accepted !== null && warning.accepted !== undefined,
      moveAfter: warning.move_after ?? null,
    });
  }
  return rows;
}

/** The ten people of the crowd. */
export const crowdOfTen: readonly Person[] = crowd;

/** The person of the crowd at this place. */
export function crowdPerson(at: number): Person {
  const person = crowd[at];
  if (person === undefined) {
    throw new RangeError(`The crowd has no person ${String(at)}.`);
  }
  return person;
}

/** Signs in these people through the API, at the same moment. */
export function signedIn(
  server: Instance,
  people: readonly Person[],
): Promise<readonly ApiSession[]> {
  return Promise.all(people.map((person) => signInThroughApi(server, person)));
}

/** The session at this place. */
export function sessionAt(sessions: readonly ApiSession[], at: number): ApiSession {
  const session = sessions[at];
  if (session === undefined) {
    throw new RangeError(`There is no session ${String(at)}.`);
  }
  return session;
}

/** The issue with this readable ID, as a person reads it. */
export async function readIssue(session: ApiSession, key: string) {
  return dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  ).issue;
}
