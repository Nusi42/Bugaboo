import { t } from "@bugaboo/messages";
import { expect } from "@playwright/test";

import { dashboardAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import {
  boxOnScreen,
  chooseByKeys,
  createDashboard,
  resultsBox,
  startChange,
  tabTo,
} from "./support/dashboards.ts";
import { test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// *Move …* and *Resize …* take the keys of the handle of their box only (CFG-03, UX §4.10): leaving the box ends them,
// so the rest of the page keeps its keys, and *Done* ends them too.

test.use({ preparation: { run: prepareWeb } });

/** Opens a dashboard of two boxes in *Change*. */
async function openTwo(page: Parameters<typeof signIn>[0], server: Parameters<typeof apiOf>[1]) {
  const made = await createDashboard(apiOf(page, server), "Keys", [
    resultsBox("First", { column: 0, row: 0, width: 6, height: 2 }),
    resultsBox("Second", { column: 6, row: 0, width: 6, height: 2 }),
  ]);
  await page.goto(dashboardAddress(made.id));
  await expect(boxOnScreen(page, "First")).toBeVisible();
  await startChange(page);
}

test("cfg_03_leaving_a_box_that_is_moved_by_keys_gives_the_page_its_keys", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openTwo(page, server);
  await chooseByKeys(page, "First", t("dashboards.move"));
  await expect(boxOnScreen(page, "First")).toHaveAttribute("data-state", "keys");
  // Out of the box to *Add box*: placing ends, and Space presses the button.
  const add = page.getByRole("button", { name: t("dashboards.add-box"), exact: true });
  await tabTo(page, add, true);
  await expect(boxOnScreen(page, "First")).not.toHaveAttribute("data-state", "keys");
  await page.keyboard.press("Space");
  await expect(page.getByRole("dialog", { name: t("dashboards.add-box") })).toBeVisible();
});

test("cfg_03_done_ends_moving_a_box_by_keys", async ({ page, server }) => {
  await signIn(page, ada);
  await openTwo(page, server);
  await chooseByKeys(page, "Second", t("dashboards.resize"));
  await expect(boxOnScreen(page, "Second")).toHaveAttribute("data-state", "keys");
  await page.getByRole("button", { name: t("dashboards.done"), exact: true }).click();
  await startChange(page);
  await expect(boxOnScreen(page, "Second")).not.toHaveAttribute("data-state", "keys");
});

test("cfg_03_tab_from_the_menu_of_a_box_out_of_it_ends_moving_it_by_keys", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openTwo(page, server);
  await chooseByKeys(page, "First", t("dashboards.move"));
  // The menu of the box is still in the box: placing goes on.
  const menu = page.getByRole("button", {
    name: t("dashboards.box-menu", { title: "First" }),
    exact: true,
  });
  await tabTo(page, menu);
  await expect(boxOnScreen(page, "First")).toHaveAttribute("data-state", "keys");
  // On past it, out of the box: placing ends.
  await tabTo(
    page,
    page.getByRole("button", { name: t("dashboards.place", { title: "Second" }), exact: true }),
  );
  await expect(boxOnScreen(page, "First")).not.toHaveAttribute("data-state", "keys");
});

test("cfg_03_a_click_elsewhere_ends_moving_a_box_by_keys", async ({ page, server }) => {
  await signIn(page, ada);
  await openTwo(page, server);
  await chooseByKeys(page, "Second", t("dashboards.move"));
  await expect(boxOnScreen(page, "Second")).toHaveAttribute("data-state", "keys");
  await page.getByRole("heading", { name: "Keys", exact: true }).click();
  await expect(boxOnScreen(page, "Second")).not.toHaveAttribute("data-state", "keys");
  // The arrow keys are the page's again: the box stays where it is.
  await page.keyboard.press("ArrowLeft");
  await expect(boxOnScreen(page, "Second")).not.toHaveAttribute("data-state", "keys");
});
