import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { dashboardAddress } from "../src/shared/dashboard-address.ts";
import { ada } from "./support/accounts.ts";
import {
  boxOnScreen,
  chooseByKeys,
  cornerOf,
  createDashboard,
  dashboardOf,
  dragBy,
  expectPlace,
  handleOf,
  holdAtTheBottomEdge,
  placesOf,
  readingTitles,
  resultsBox,
  startChange,
} from "./support/dashboards.ts";
import { boxReadsFrom, waitForBoxAnswers } from "./support/dashboard-reads.ts";
import { test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { expectStill, landmarkOf, movementWhile } from "./support/movement.ts";

// Placing the boxes of a dashboard on a computer (CFG-03, UX §4.10, §6.4): by drag and by the corner, by the keys of
// *Move …* and *Resize …* to the same places, and nothing moves under the cursor while a box is dragged.

test.use({ preparation: { run: prepareWeb } });

/** Two boxes side by side, and one under the first. */
const layout = [
  resultsBox("First", { column: 0, row: 0, width: 6, height: 2 }),
  resultsBox("Second", { column: 6, row: 0, width: 6, height: 2 }),
  resultsBox("Third", { column: 0, row: 2, width: 6, height: 2 }),
] as const;

/** Opens a new dashboard of the layout in *Change*; its ID. */
async function openInChange(page: Page, server: Parameters<typeof apiOf>[1]): Promise<string> {
  const made = await createDashboard(apiOf(page, server), "Placing", layout);
  await page.goto(dashboardAddress(made.id));
  await expect(boxOnScreen(page, "First")).toBeVisible();
  // The first reads of the boxes are in before anything is dragged: their grey shapes are a first read, not a drag.
  await waitForBoxAnswers(page);
  await startChange(page);
  return made.id;
}

test("cfg_03_a_box_is_placed_by_drag_and_resized_by_its_corner", async ({ page, server }) => {
  await signIn(page, ada);
  const id = await openInChange(page, server);
  const api = apiOf(page, server);
  // *Third* dragged up onto *Second*'s column goes under it: packing keeps no free row.
  await dragBy(page, handleOf(page, "Third"), { columns: 6, rows: 0 });
  await expectPlace(api, id, "Third", { column: 6, row: 2, width: 6, height: 2 });
  // *First* made a row higher by its corner.
  await dragBy(page, cornerOf(page, "First"), { columns: 0, rows: 1 });
  await expectPlace(api, id, "First", { column: 0, row: 0, width: 6, height: 3 });
});

test("cfg_03_move_and_resize_do_what_a_drag_does", async ({ page, server }) => {
  await signIn(page, ada);
  const api = apiOf(page, server);
  // A drag of *Third* one column to the right, and of the corner of *First* a column wider.
  const dragged = await openInChange(page, server);
  await dragBy(page, handleOf(page, "Third"), { columns: 1, rows: 0 });
  await expectPlace(api, dragged, "Third", { column: 1, row: 2, width: 6, height: 2 });
  await dragBy(page, cornerOf(page, "First"), { columns: 1, rows: 0 });
  await expect.poll(async () => (await dashboardOf(api, dragged)).version).toBeGreaterThan(2);
  const byDrag = placesOf(await dashboardOf(api, dragged));
  // The same with the keyboard only: *Move …* and an arrow, *Resize …* and an arrow, each kept with Enter.
  const keyed = await openInChange(page, server);
  await chooseByKeys(page, "Third", t("dashboards.move"));
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await expectPlace(api, keyed, "Third", { column: 1, row: 2, width: 6, height: 2 });
  await chooseByKeys(page, "First", t("dashboards.resize"));
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await expect.poll(async () => placesOf(await dashboardOf(api, keyed))).toEqual(byDrag);
  // Escape goes back: nothing is kept.
  await chooseByKeys(page, "Second", t("dashboards.move"));
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("Escape");
  await expect.poll(async () => placesOf(await dashboardOf(api, keyed))).toEqual(byDrag);
});

test("cfg_03_nothing_moves_under_the_cursor_while_a_box_is_dragged", async ({ page, server }) => {
  await signIn(page, ada);
  await openInChange(page, server);
  const landmarks = [
    await landmarkOf(boxOnScreen(page, "First"), "first"),
    await landmarkOf(boxOnScreen(page, "Second"), "second"),
    await landmarkOf(boxOnScreen(page, "Third"), "third"),
  ];
  // While *Third* is dragged over *Second*, every box stays where it is; only its shadow shows where it lands. No box
  // reads its answer again, neither while it is dragged nor once it is let go: a change of places changes no answer.
  const reads = boxReadsFrom(page);
  const dragging = await movementWhile(page, landmarks, async () => {
    await dragBy(page, handleOf(page, "Third"), { columns: 6, rows: -2 }, false);
  });
  expectStill(dragging);
  await page.mouse.up();
  await expect(page.getByText(t("dashboards.moved", { title: "Third" }))).toBeVisible();
  expect(reads.stop()).toEqual([]);
});

test("cfg_03_a_box_dragged_to_the_bottom_edge_scrolls_and_lands_below_what_was_shown", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  // Three boxes, each the whole width and high, so the grid is higher than the window.
  const made = await createDashboard(apiOf(page, server), "Tall", [
    resultsBox("Top", { column: 0, row: 0, width: 12, height: 6 }),
    resultsBox("Middle", { column: 0, row: 6, width: 12, height: 6 }),
    resultsBox("Bottom", { column: 0, row: 12, width: 12, height: 6 }),
  ]);
  await page.goto(dashboardAddress(made.id));
  await expect(boxOnScreen(page, "Top")).toBeVisible();
  await startChange(page);
  await holdAtTheBottomEdge(page, handleOf(page, "Top"));
  // The page scrolled while the box was held at the edge without moving, and the box went past the last one, which the
  // window did not show when the drag started.
  await expect
    .poll(async () => readingTitles(await dashboardOf(apiOf(page, server), made.id)))
    .toEqual(["Middle", "Bottom", "Top"]);
  expect(await page.getByRole("main").evaluate((main) => main.scrollTop)).toBeGreaterThan(0);
});
