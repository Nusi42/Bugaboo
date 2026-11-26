import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import type { ApiSession } from "./api.ts";
import { stepsFrom, type GridSteps } from "../../src/shared/dashboard-grid-steps.ts";
import { personName } from "../../src/shared/person.ts";
import { boxOf } from "./geometry.ts";
import { partIn } from "./parts.ts";
import { inOrder } from "./sequence.ts";

/** A box as a test asks for it: a kind, a place, a title and what it reads. */
export type AskedBox = Schema<"BoxBody">;

/** A box of search results at a place, reading every issue. */
export function resultsBox(
  title: string,
  place: Pick<AskedBox, "column" | "row" | "width" | "height">,
): AskedBox {
  return { kind: "search-results", title, search_text: "", ...place };
}

/** Makes a dashboard of the person of the session, with these boxes; its ID. */
export async function createDashboard(
  api: ApiSession,
  name: string,
  boxes: readonly AskedBox[],
): Promise<Schema<"DashboardBody">> {
  return dataOf(await api.client.POST("/api/v1/dashboards", { body: { name, boxes: [...boxes] } }));
}

/** The dashboard as the person of the session sees it now. */
export async function dashboardOf(api: ApiSession, id: string): Promise<Schema<"DashboardBody">> {
  return dataOf(
    await api.client.GET("/api/v1/dashboards/{dashboard}", { params: { path: { dashboard: id } } }),
  );
}

/** The place of a box of the dashboard, by its title. */
export function placeOf(
  dashboard: Schema<"DashboardBody">,
  title: string,
): Pick<AskedBox, "column" | "row" | "width" | "height"> | undefined {
  const box = dashboard.boxes.find((each) => each.title === title);
  return box === undefined
    ? undefined
    : { column: box.column, row: box.row, width: box.width, height: box.height };
}

/** The places of every box of the dashboard by title, for comparing two dashboards. */
export function placesOf(dashboard: Schema<"DashboardBody">): Readonly<Record<string, unknown>> {
  return Object.fromEntries(
    dashboard.boxes.map((box) => [box.title, placeOf(dashboard, box.title)]),
  );
}

/** A box on the screen, by its title. */
export function boxOnScreen(page: Page, title: string): Locator {
  return partIn(page, "dashboard-box").filter({
    has: page.getByRole("heading", { name: title, exact: true }),
  });
}

/** The handle a pointer drags a box by (its title), and the corner it resizes it by. */
export function handleOf(page: Page, title: string): Locator {
  return partIn(boxOnScreen(page, title), "dashboard-box.handle");
}

export function cornerOf(page: Page, title: string): Locator {
  return partIn(boxOnScreen(page, title), "dashboard-box.corner");
}

/** Starts *Change* of the dashboard that is open. */
export async function startChange(page: Page): Promise<void> {
  await page.getByRole("button", { name: t("dashboards.change"), exact: true }).click();
  await expect(page.getByText(t("dashboards.changing"), { exact: true })).toBeVisible();
}

/** How far one column and one row of the grid are on the screen: the one measure of the screen (`grid-steps.ts`). */
export async function gridSteps(page: Page): Promise<GridSteps> {
  const measure = await partIn(page, "dashboard-grid").evaluate((grid) => {
    const style = getComputedStyle(grid);
    return {
      width: grid.clientWidth,
      columnGap: style.columnGap,
      rowHeight: style.gridAutoRows,
      rowGap: style.rowGap,
      rows: style.gridTemplateRows,
    };
  });
  return stepsFrom(measure);
}

/** Drags from the middle of what the locator shows by a number of columns and rows, and lets go. */
export async function dragBy(
  page: Page,
  from: Locator,
  steps: { readonly columns: number; readonly rows: number },
  release = true,
): Promise<void> {
  await from.scrollIntoViewIfNeeded();
  const box = await from.boundingBox();
  if (box === null) {
    throw new TypeError("Nothing to drag on the screen.");
  }
  const step = await gridSteps(page);
  const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
  await page.mouse.move(x, y);
  await page.mouse.down();
  const moves = 8;
  await inOrder(
    Array.from(
      { length: moves },
      (_, at) => () =>
        page.mouse.move(
          x + (steps.columns * step.column * (at + 1)) / moves,
          y + (steps.rows * step.row * (at + 1)) / moves,
        ),
    ),
  );
  if (release) {
    await page.mouse.up();
  }
}

/**
 * Drags what the locator shows to the bottom edge of the window and holds it there, without moving, until the page has
 * scrolled to its end (UX §4.10), then lets go.
 */
export async function holdAtTheBottomEdge(page: Page, from: Locator): Promise<void> {
  const box = await from.boundingBox();
  const window = page.viewportSize();
  if (box === null || window === null) {
    throw new TypeError("Nothing to drag on the screen.");
  }
  const x = box.x + box.width / 2;
  await page.mouse.move(x, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(x, window.height / 2);
  await page.mouse.move(x, window.height - 2);
  const main = page.getByRole("main");
  await expect
    .poll(() =>
      main.evaluate(
        (scrolled) => scrolled.scrollHeight - scrolled.clientHeight - scrolled.scrollTop,
      ),
    )
    .toBeLessThan(1);
  await page.mouse.up();
}

/** Waits until the dashboard the server keeps has the places the test expects of one box. */
export async function expectPlace(
  api: ApiSession,
  id: string,
  title: string,
  place: Pick<AskedBox, "column" | "row" | "width" | "height">,
): Promise<void> {
  await expect.poll(async () => placeOf(await dashboardOf(api, id), title)).toEqual(place);
}

/** Shares a dashboard with a project, by its short name. */
export async function shareWithProject(
  api: ApiSession,
  id: string,
  shortName: string,
): Promise<void> {
  const projects = dataOf(
    await api.client.GET("/api/v1/projects", { params: { query: { limit: 100 } } }),
  );
  const project = projects.items.find((each) => each.short_name === shortName)?.id;
  if (project === undefined || project === null) {
    throw new TypeError(`The project ${shortName} is not there.`);
  }
  await api.client.POST("/api/v1/dashboards/{dashboard}/shares", {
    params: { path: { dashboard: id } },
    body: { kind: "project", id: project },
  });
}

/** The title of a ready-made box, in the words of the catalog of the server. */
export async function readyMadeTitle(api: ApiSession, which: Schema<"ReadyMade">): Promise<string> {
  const ready = dataOf(await api.client.GET("/api/v1/dashboards/ready-made-boxes"));
  const found = ready.boxes.find((each) => each.which === which);
  if (found === undefined) {
    throw new TypeError(`There is no ready-made box ${which}.`);
  }
  return found.box.title;
}

/** *Show as table* of a chart of a box: a person presses its words, which stand beside the track of the switch. */
export async function showAsTable(box: Locator): Promise<void> {
  await box.getByText(t("dashboards.as-table"), { exact: true }).click();
}

/** The search text a box reads, as an address takes it. */
export function searchOf(box: AskedBox): string {
  return encodeURIComponent(box.search_text ?? "");
}

/**
 * A dashboard shared with WEB whose first box (*Due soon*, on top) reads the due date, which a guest may not read, and
 * whose second box (one number of every issue) stands under it; its ID and the titles of the two boxes.
 */
export async function createGuestTeam(
  api: ApiSession,
): Promise<{ readonly id: string; readonly hidden: string; readonly counted: string }> {
  const hidden = await readyMadeTitle(api, "due-soon");
  const counted = "Counted";
  const made = await createDashboard(api, "Team", [
    {
      ...resultsBox(hidden, { column: 0, row: 0, width: 12, height: 2 }),
      search_text: "due date is this week",
    },
    {
      kind: "numbers-and-chart",
      title: counted,
      search_text: "",
      shape: "number",
      column: 0,
      row: 2,
      width: 6,
      height: 2,
    },
  ]);
  await shareWithProject(api, made.id, "WEB");
  return { id: made.id, hidden, counted };
}

/** What a viewer sees of a dashboard by themselves: how many issues their own totals count, and who made it. */
export async function viewOf(
  api: ApiSession,
  id: string,
): Promise<{ readonly issues: number; readonly maker: string }> {
  const own = dataOf(await api.client.POST("/api/v1/search/totals", { body: {} }));
  const seen = await dashboardOf(api, id);
  return { issues: own.all.issues, maker: personName(seen.maker) };
}

/** Where across each element the locator finds starts, in whole pixels. */
export async function startsOf(shown: Locator): Promise<readonly number[]> {
  return Promise.all((await shown.all()).map(async (each) => Math.round((await boxOf(each)).x)));
}

/** The titles of the boxes of a dashboard in the order they are read: row by row, and in a row from the start. */
export function readingTitles(dashboard: Schema<"DashboardBody">): readonly string[] {
  return dashboard.boxes
    .toSorted((one, other) => one.row - other.row || one.column - other.column)
    .map((box) => box.title);
}

/** How many presses of Tab a test tries before it says that a control cannot be reached by the keyboard. */
const TAB_PRESSES_MOST = 60;

/** Presses Tab (or Shift+Tab) until the control has the focus, as a person with a keyboard reaches it. */
export async function tabTo(page: Page, control: Locator, backwards = false): Promise<void> {
  const key = backwards ? "Shift+Tab" : "Tab";
  const presses = Array.from({ length: TAB_PRESSES_MOST }, () => async () => {
    if (!(await control.evaluate((element) => element === document.activeElement))) {
      await page.keyboard.press(key);
    }
  });
  await inOrder(presses);
  await expect(control).toBeFocused();
}

/** Opens the menu of a box by the keyboard alone and chooses one of its actions; the handle then has the keys. */
export async function chooseByKeys(page: Page, title: string, action: string): Promise<void> {
  await tabTo(
    page,
    page.getByRole("button", { name: t("common.actions-for", { name: title }), exact: true }),
  );
  await page.keyboard.press("Enter");
  await tabTo(page, page.getByRole("button", { name: action, exact: true }));
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("button", { name: t("dashboards.place", { title }), exact: true }),
  ).toBeFocused();
}
