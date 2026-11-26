import { t } from "@bugaboo/messages";
import { expect } from "@playwright/test";

import { dashboardAddress } from "../src/shared/dashboard-address.ts";
import { ada } from "./support/accounts.ts";
import {
  boxOnScreen,
  createDashboard,
  dashboardOf,
  readingTitles,
  resultsBox,
  startChange,
  startsOf,
} from "./support/dashboards.ts";
import { test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { topsOf } from "./support/geometry.ts";
import { prepareWeb } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";

// A dashboard on a phone (CFG-03, ADR-0050, decision 3): the boxes stand in one column in the order of the grid (row,
// then column), and *Change* moves a box with *Move up* and *Move down* instead of a drag.

test.use({ preparation: { run: prepareWeb } });

test("cfg_03_the_phone_shows_boxes_in_one_column", async ({ page, server }) => {
  await signIn(page, ada);
  const api = apiOf(page, server);
  const made = await createDashboard(api, "Pocket", [
    resultsBox("Right", { column: 6, row: 0, width: 6, height: 2 }),
    resultsBox("Left", { column: 0, row: 0, width: 6, height: 2 }),
    resultsBox("Under", { column: 0, row: 2, width: 12, height: 2 }),
  ]);
  await page.goto(dashboardAddress(made.id));
  await expect(boxOnScreen(page, "Under")).toBeVisible();
  // One column: every box starts at the same place across, in the reading order of the grid.
  const boxes = partIn(page, "dashboard-box");
  await expect(boxes.getByRole("heading", { level: 2 })).toHaveText(["Left", "Right", "Under"]);
  const tops = await topsOf(boxes);
  expect(tops).toEqual(tops.toSorted((one, other) => one - other));
  const starts = await startsOf(boxes);
  expect(new Set(starts).size).toBe(1);
  // *Change* has *Move down* instead of a drag; there is no handle to drag.
  await startChange(page);
  await expect(partIn(page, "dashboard-box.handle")).toHaveCount(0);
  await page
    .getByRole("button", { name: t("common.actions-for", { name: "Left" }), exact: true })
    .click();
  await page.getByRole("button", { name: t("dashboards.move-down"), exact: true }).click();
  await expect(boxes.getByRole("heading", { level: 2 })).toHaveText(["Right", "Left", "Under"]);
  // The order is kept: the places the server keeps read in the new order, row by row.
  await expect
    .poll(async () => readingTitles(await dashboardOf(api, made.id)))
    .toEqual(["Right", "Left", "Under"]);
});
