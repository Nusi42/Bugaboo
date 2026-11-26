import { dataOf } from "@bugaboo/api-client";
import { expect, type Page } from "@playwright/test";

import { savedSearchAddress } from "../../src/shared/paths.ts";
import { ada, signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { createIssue, issueOf, prepareWeb, versionOf } from "./issues.ts";
import { inOrder } from "./sequence.ts";
import { calendarOf, weekDays } from "./calendar-places.ts";
import { changeIssue } from "./timeline.ts";
import { createOwnDates, setOwnValues } from "./timeline-api.ts";

// The scene of the tests of the calendar (PLAN-09, UX §4.16): the project WEB of the tests of issues,
// its issues with due dates in the week of today, a goal over one of them, a release dated today and
// a saved search that shows WEB as a month.

/** The issues of the scene by what they are for. WEB-1 to WEB-3 are the issues of `prepareWeb`. */
export const scene = {
  /** On the crowded day, with WEB-11 to WEB-13: more than a day of the month shows. */
  crowd: ["WEB-1", "WEB-2", "WEB-3", "WEB-11", "WEB-12", "WEB-13"],
  /** The first of them: it stands on another day than the issues the tests move. */
  crowdFirst: "WEB-1",
  dragged: "WEB-4",
  typed: "WEB-5",
  startOnly: "WEB-6",
  ownOnly: "WEB-7",
  locked: "WEB-8",
  goal: "WEB-9",
  child: "WEB-10",
} as const;

/** The title of the goal and the name of the release. */
export const goalTitle = "Mobile app 2.0";
export const releaseName = "2.5";

/** The titles of the issues made after WEB-3, in the order of their IDs (WEB-4 on). */
const titles = [
  "Drag me",
  "Keys only",
  "Start dated",
  "Own dated",
  "Locked",
  goalTitle,
  "Child of the goal",
  "Crowd a",
  "Crowd b",
  "Crowd c",
] as const;

/** The own date field of the scene: its search word, which a view names in *Dates from*. */
export const ownDateWord = "begins";

/** The ID of the own date field of the scene, once the server is ready. */
export const ownFields = { begins: "" };

/** The ID of the saved search *Calendar*, which shows WEB as a month. */
let calendarSearch = "";

/** Puts an issue under a goal. */
async function under(session: ApiSession, key: string, goal: string): Promise<void> {
  const version = await versionOf(session, await issueOf(session, key));
  dataOf(
    await session.client.PUT("/api/v1/issues/{issue}/parent", {
      params: { path: { issue: key } },
      body: { version, parent: goal },
    }),
  );
}

/** Gives the issues of the scene their dates. */
async function giveDates(server: Instance, admin: ApiSession, ownField: string): Promise<void> {
  const { today, crowded } = weekDays();
  const due = (key: string, day: string) => () => changeIssue(admin, key, { due_date: day });
  await inOrder([
    ...scene.crowd.map((key) => due(key, crowded)),
    ...[scene.dragged, scene.typed, scene.locked, scene.goal, scene.child].map((key) =>
      due(key, today),
    ),
    () => changeIssue(admin, scene.startOnly, { start_date: today }),
    () => setOwnValues(server, scene.ownOnly, { [ownField]: today }),
  ]);
}

/** Gets the server ready: WEB, its issues with dates, the goal, the release, the saved search. */
export async function prepareCalendar(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  await inOrder(titles.map((title) => () => createIssue(admin, "WEB", title)));
  const fields = await createOwnDates(server);
  ownFields.begins = fields.begins;
  await giveDates(server, admin, fields.begins);
  await under(admin, scene.child, scene.goal);
  dataOf(
    await admin.client.POST("/api/v1/projects/{project}/releases", {
      params: { path: { project: "WEB" } },
      body: { name: releaseName, release_date: weekDays().today },
    }),
  );
  const saved = dataOf(
    await admin.client.POST("/api/v1/saved-searches", {
      body: {
        name: "Calendar",
        search_text: "project is WEB",
        presentation: {
          view: "calendar",
          columns: [],
          group_by: null,
          calendar: { dates_field: null, span: "month" },
        },
      },
    }),
  );
  calendarSearch = saved.id ?? "";
  await admin.dispose();
}

/** Opens the saved search *Calendar* as a month. */
export async function openCalendar(page: Page): Promise<void> {
  await page.goto(savedSearchAddress(calendarSearch));
  await expect(calendarOf(page)).toBeVisible();
}

/** Opens the issues of WEB as a calendar, with these more parts of the address: the page of a project reads its releases too. */
export async function openProjectCalendar(page: Page, more = "&span=week"): Promise<void> {
  await page.goto(`/p/WEB/issues?show=calendar${more}`);
  await expect(calendarOf(page)).toBeVisible();
}

/** Where the person that signed in with this address keeps the due date of an issue. */
export async function dueOf(session: ApiSession, key: string): Promise<string | null> {
  const read = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  return read.issue.due_date ?? null;
}
