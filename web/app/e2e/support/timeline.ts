import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { savedSearchAddress } from "../../src/shared/paths.ts";
import { ada, signInThroughApi } from "./accounts.ts";
import type { ApiSession } from "./api.ts";
import { createIssue, issueOf, prepareManyWeb, prepareWeb, versionOf } from "./issues.ts";
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
  pressed: "WEB-12",
  oneDate: "WEB-13",
  keyThenDrag: "WEB-14",
  twice: "WEB-15",
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
  { key: planned.pressed, title: "Pressed fast", from: 4, to: 6 },
  { key: planned.oneDate, title: "Only a start", from: 5 },
  { key: planned.keyThenDrag, title: "Key then drag", from: 1, to: 2 },
  { key: planned.twice, title: "Moved twice", from: 2, to: 3 },
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

/**
 * Every look a bar can have, for the pictures and the eye: in progress with half of it done and a
 * High edge, done, and Critical.
 */
async function lookOfTheScene(admin: ApiSession): Promise<void> {
  await changeIssue(admin, planned.first, {
    status: "in-progress",
    priority: "high",
    estimated_time: 480,
    time_left: 240,
  });
  await changeIssue(admin, planned.dotFrom, { status: "done" });
  await changeIssue(admin, planned.dotTo, { priority: "critical" });
}

/** The ID of the saved search *Plan*, which shows WEB as a timeline grouped by goal. */
let planSearch = "";

/** Gets the server ready: WEB, its issues with dates, the goal, a dependency, the saved search. */
export async function prepareTimeline(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  await inOrder(scene.map((each) => () => createIssue(admin, "WEB", each.title)));
  await giveDates(admin, planned.first, 1, 3);
  // WEB-2 starts before WEB-1 ends, although it waits for it: the plan warns of a Delay.
  await giveDates(admin, planned.waits, 2, 8);
  // The issues of the tray keep no dates.
  await inOrder(
    scene
      .filter((each) => each.from !== undefined || each.to !== undefined)
      .map((each) => () => giveDates(admin, each.key, each.from, each.to)),
  );
  await under(admin, planned.first, planned.goal);
  await under(admin, planned.waits, planned.goal);
  await relateBlocking(admin, planned.first, planned.waits);
  await lookOfTheScene(admin);
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

/** The issues `prepareWeb` makes before any other: WEB-1 to WEB-3. */
const FIRST_ISSUES = 3;

/**
 * Gets the server ready for a timeline longer than its window: WEB with `dated` more issues that have dates, a few days
 * apart, and `undated` more without, for *Not scheduled yet* (with the three of `prepareWeb`).
 */
export async function prepareLongTimeline(
  server: Instance,
  dated: number,
  undated: number,
): Promise<void> {
  await prepareManyWeb(server, dated + undated);
  const admin = await signInThroughApi(server, ada);
  await inOrder(
    Array.from(
      { length: dated },
      (_, at) => () =>
        giveDates(admin, `WEB-${String(FIRST_ISSUES + at + 1)}`, at % 7, (at % 7) + 2),
    ),
  );
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
  return page
    .locator('[data-part="timeline-bar"]')
    .filter({ has: page.getByText(key, { exact: true }) });
}

/** The button that fills the bar: what is pressed, dragged and moved by keys. */
export function pressOf(page: Page, key: string): Locator {
  return barOf(page, key).locator('[data-part="timeline-bar.press"]');
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

/** What a move says, up to the dates it names: "WEB-5 moved 1 week later, now". */
export function movedLater(key: string, amount: string): string {
  return t("timeline.moved-later", { key, amount, dates: "" }).trim();
}

/**
 * What the edge of the days on screen cuts on the chart (UX §4.7): the label of a bar that is on
 * screen, and the name of a unit of the scale that is seen in part. A label or a name that is wholly
 * out of sight is not cut; a label the box shows must show whole, and a name must show whole after
 * the names of the rows and before the end of its own unit.
 */
export function cutByTheEdge(page: Page): Promise<readonly string[]> {
  return page.evaluate(() => {
    const box = document.querySelector('[data-part="timeline-scroll"]');
    const corner = document.querySelector('[data-part="timeline-head.corner"]');
    if (box === null || corner === null) {
      return ["no chart"];
    }
    const start = corner.getBoundingClientRect().right;
    const end = box.getBoundingClientRect().left + box.clientLeft + box.clientWidth;
    // Half a pixel either way: the browser places parts on fractions of a pixel.
    const slack = 0.5;
    const seen = (rect: Readonly<DOMRect>) =>
      rect.width > 0 && rect.right > start && rect.left < end;
    const whole = (rect: Readonly<DOMRect>, from: number, to: number) =>
      rect.left >= from - slack && rect.right <= to + slack;
    const cut: string[] = [];
    for (const bar of document.querySelectorAll('[data-part="timeline-bar"]')) {
      const label = bar.querySelector('[data-part="timeline-bar.label"]');
      if (label !== null && seen(bar.getBoundingClientRect())) {
        const rect = label.getBoundingClientRect();
        if (!whole(rect, start, end)) {
          cut.push(label.textContent);
        }
      }
    }
    for (const cell of document.querySelectorAll('[data-part="day-scale.cell"]')) {
      const words = document.createRange();
      words.selectNodeContents(cell);
      const rect = words.getBoundingClientRect();
      const own = cell.getBoundingClientRect();
      // The end of the box cuts a name as it cuts the days of its unit, which go on past it; the
      // names of the rows at the start and the end of its own unit never do.
      if (seen(rect) && !whole(rect, Math.max(start, own.left), own.right)) {
        cut.push(cell.textContent);
      }
    }
    return cut;
  });
}
