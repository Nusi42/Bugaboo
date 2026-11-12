import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { savedSearchAddress } from "../../src/shared/paths.ts";
import { ada, signInThroughApi, type Person } from "./accounts.ts";
import type { ApiSession } from "./api.ts";
import { createIssue, issueOf, prepareWeb, versionOf } from "./issues.ts";
import { BLOCKS } from "./relate.ts";
import { inOrder } from "./sequence.ts";
import type { Instance } from "./server.ts";

// The scene of the tests of the timeline (PLAN-01 … PLAN-04, UX §4.7): the project WEB of the tests
// of issues, its issues given dates around today, a goal over two of them, one that must finish
// before the other, and issues without dates for *Not scheduled yet*.

/** The day `days` after today on this computer, as the calendar writes it (`2027-03-01`). */
export function dayFromToday(days: number): string {
  const day = new Date();
  day.setDate(day.getDate() + days);
  const month = String(day.getMonth() + 1).padStart(2, "0");
  const date = String(day.getDate()).padStart(2, "0");
  return `${String(day.getFullYear())}-${month}-${date}`;
}

/** The issues of the scene by what they are for. WEB-1 to WEB-3 are the issues of `prepareWeb`. */
export const planned = {
  first: "WEB-1",
  waits: "WEB-2",
  undated: "WEB-3",
  goal: "WEB-4",
  dragged: "WEB-5",
  keys: "WEB-6",
  dotFrom: "WEB-7",
  dotTo: "WEB-8",
  found: "WEB-9",
  trayKeys: "WEB-10",
  ownDates: "WEB-11",
} as const;

/** The title of the goal. */
export const goalTitle = "Mobile app 2.0";

/** The own date fields the view of own dates uses, by their names and search words. */
export const ownDates = { begins: "Begins", ends: "Ends" } as const;

/** The issues the scene makes after WEB-3, in the order of their IDs, with their dates from today. */
const scene: readonly {
  readonly key: string;
  readonly title: string;
  readonly from?: number;
  readonly to?: number;
}[] = [
  { key: planned.goal, title: goalTitle, to: 12 },
  { key: planned.dragged, title: "Drag me", from: 2, to: 4 },
  { key: planned.keys, title: "Keys only", from: 3, to: 5 },
  { key: planned.dotFrom, title: "Dot start", from: 1, to: 2 },
  { key: planned.dotTo, title: "Dot end", from: 6, to: 7 },
  { key: planned.found, title: "Found by the finder", from: 8, to: 9 },
  { key: planned.trayKeys, title: "Tray by keys" },
  { key: planned.ownDates, title: "Own dates" },
];

/** Gives an issue dates (or other members), on the version it has. */
export async function changeIssue(
  session: ApiSession,
  key: string,
  body: Readonly<Record<string, unknown>>,
): Promise<void> {
  const version = await versionOf(session, await issueOf(session, key));
  dataOf(
    await session.client.PATCH("/api/v1/issues/{issue}", {
      params: { path: { issue: key } },
      body: { version, ...body },
    }),
  );
}

/** Gives an issue its start and due date, `from` and `to` days after today. */
async function giveDates(session: ApiSession, key: string, from?: number, to?: number) {
  await changeIssue(session, key, {
    ...(from === undefined ? {} : { start_date: dayFromToday(from) }),
    ...(to === undefined ? {} : { due_date: dayFromToday(to) }),
  });
}

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

/** The ID of the starting kind *blocks*, the one with *Must finish first*. */
async function blocksKind(session: ApiSession): Promise<string> {
  const kinds = dataOf(await session.client.GET("/api/v1/relation-kinds"));
  const found = kinds.items.find((kind) => kind.forward === BLOCKS)?.id;
  if (found === undefined || found === null) {
    throw new TypeError("The server has no kind blocks.");
  }
  return found;
}

/** Makes `first` block `waiting`: `first` must finish first. */
export async function relateBlocking(session: ApiSession, first: string, waiting: string) {
  dataOf(
    await session.client.POST("/api/v1/issues/{issue}/relations", {
      params: { path: { issue: first } },
      body: { other: waiting, relation_kind: await blocksKind(session), direction: "forward" },
    }),
  );
}

/** The ID of the saved search *Plan*, which shows WEB as a timeline grouped by goal. */
let planSearch = "";

/** Gets the server ready: WEB, its issues with dates, the goal, a dependency, the saved search. */
export async function prepareTimeline(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  await inOrder(scene.map((each) => () => createIssue(admin, "WEB", each.title)));
  await giveDates(admin, planned.first, 1, 3);
  await giveDates(admin, planned.waits, 5, 8);
  // The issues of the tray keep no dates.
  await inOrder(
    scene
      .filter((each) => each.from !== undefined || each.to !== undefined)
      .map((each) => () => giveDates(admin, each.key, each.from, each.to)),
  );
  await under(admin, planned.first, planned.goal);
  await under(admin, planned.waits, planned.goal);
  await relateBlocking(admin, planned.first, planned.waits);
  const saved = dataOf(
    await admin.client.POST("/api/v1/saved-searches", {
      body: {
        name: "Plan",
        search_text: "project is WEB",
        presentation: {
          view: "timeline",
          columns: [],
          group_by: "parent",
          timeline: { start_field: null, end_field: null, plan_by_effort: false, zoom: "week" },
        },
      },
    }),
  );
  planSearch = saved.id ?? "";
  await admin.dispose();
}

/** Opens the saved search *Plan* as its timeline. */
export async function openPlan(page: Page): Promise<void> {
  await page.goto(savedSearchAddress(planSearch));
  await expect(chart(page)).toBeVisible();
}

/** Opens the search of WEB as a timeline, with these more parts of the address. */
export async function openTimeline(page: Page, more = ""): Promise<void> {
  await page.goto(`/search?s=${encodeURIComponent("project is WEB")}&show=timeline${more}`);
  await expect(chart(page)).toBeVisible();
}

/** The rows of the chart. */
export function chart(page: Page): Locator {
  return page.getByRole("list", { name: t("timeline.chart"), exact: true });
}

/** The bar of the issue with this readable ID. */
export function barOf(page: Page, key: string): Locator {
  return page.locator('[data-part="timeline-bar"]').filter({ hasText: key });
}

/** The button that fills the bar: what is pressed, dragged and moved by keys. */
export function pressOf(page: Page, key: string): Locator {
  return barOf(page, key).locator('[data-part="timeline-bar.press"]');
}

/** The dates an issue keeps, as a person reads them. */
export async function datesAs(server: Instance, person: Person, key: string) {
  const session = await signInThroughApi(server, person);
  const read = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  await session.dispose();
  return { start: read.issue.start_date ?? null, due: read.issue.due_date ?? null };
}

/** The internal IDs of issues, as the administrator reads them. */
export async function idsOf(server: Instance, keys: readonly string[]): Promise<readonly string[]> {
  const admin = await signInThroughApi(server, ada);
  const ids = await Promise.all(keys.map(async (key) => (await issueOf(admin, key)).id));
  await admin.dispose();
  return ids;
}

/** The own date fields *Begins* and *Ends*, made by the administrator: their IDs. */
export async function createOwnDates(
  server: Instance,
): Promise<{ readonly begins: string; readonly ends: string }> {
  const admin = await signInThroughApi(server, ada);
  const create = async (name: string): Promise<string> => {
    const made = dataOf(
      await admin.client.POST("/api/v1/fields", {
        body: { name, kind: "date", applies_to: "issues" },
      }),
    );
    return made.id ?? "";
  };
  const begins = await create(ownDates.begins);
  const ends = await create(ownDates.ends);
  await admin.dispose();
  return { begins, ends };
}

/** Gives an issue the values of these own fields, as the administrator. */
export async function setOwnValues(
  server: Instance,
  key: string,
  fields: Readonly<Record<string, string>>,
): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  await changeIssue(admin, key, { fields });
  await admin.dispose();
}

/** The value an issue has in an own field, as the administrator reads it. */
export async function ownValueOf(server: Instance, key: string, field: string): Promise<unknown> {
  const admin = await signInThroughApi(server, ada);
  const read = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  await admin.dispose();
  return read.issue.own_fields?.[field];
}

/** The issues related to an issue, as text, as the administrator reads them. */
export async function relationsOf(server: Instance, key: string): Promise<string> {
  const admin = await signInThroughApi(server, ada);
  const relations = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}/related-issues", {
      params: { path: { issue: key } },
    }),
  );
  await admin.dispose();
  return JSON.stringify(relations);
}

/** The arrow from the issue with the first internal ID to the one with the second. */
export function arrowOf(page: Page, ids: readonly (string | undefined)[]): Locator {
  const [first = "", waiting = ""] = ids;
  return page.locator(`[data-arrow="${first}>${waiting}"]`);
}

/** The handle that drags an issue of *Not scheduled yet* onto the chart. */
export function trayHandle(scope: Locator, key: string): Locator {
  return scope.locator(
    `[data-part="draggable.handle"][aria-label="${t("timeline.drag-issue", { key })}"]`,
  );
}

/** Drags the middle of `what` across by `pixels`, with the pointer. */
export async function dragAcross(page: Page, what: Locator, pixels: number): Promise<void> {
  const box = await what.boundingBox();
  if (box === null) {
    throw new TypeError("Nothing to drag.");
  }
  const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + pixels / 2, y, { steps: 4 });
  await page.mouse.move(x + pixels, y, { steps: 4 });
  await page.mouse.up();
}
