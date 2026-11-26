import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect } from "@playwright/test";

import { dashboardAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { boxOnScreen, createDashboard, dashboardOf, readyMadeTitle } from "./support/dashboards.ts";
import { test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// Dashboards on screen (CFG-03, ADR-0050): a dashboard built from the ready-made boxes with their previews, and
// *Customize* on *Home*.

test.use({ preparation: { run: prepareWeb } });

test("cfg_03_a_dashboard_is_built_from_ready_made_boxes_with_previews", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  const api = apiOf(page, server);
  const made = await createDashboard(api, "Release board", []);
  const title = await readyMadeTitle(api, "issues-by-status-group");
  await page.goto(dashboardAddress(made.id));
  // An empty dashboard of its maker opens in *Change*, with *Add box*.
  await page.getByRole("button", { name: t("dashboards.add-box"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("dashboards.add-box") });
  const choice = window.getByRole("listitem").filter({
    has: page.getByRole("heading", { name: title, exact: true }),
  });
  // The preview is the box as it reads for the person: its chart, with the table that screen readers read.
  await expect(choice.getByRole("table", { includeHidden: true })).toHaveCount(1);
  await expect(choice.getByRole("rowheader", { includeHidden: true }).first()).toBeAttached();
  await choice
    .getByRole("button", { name: t("dashboards.add-this", { title }), exact: true })
    .click();
  await expect(boxOnScreen(page, title)).toBeVisible();
  await expect
    .poll(async () => (await dashboardOf(api, made.id)).boxes.map((box) => box.title))
    .toEqual([title]);
});

test("cfg_03_customize_on_home_changes_the_persons_dashboard", async ({ page, server }) => {
  await signIn(page, ada);
  const api = apiOf(page, server);
  const [timer, open, due] = [
    await readyMadeTitle(api, "my-timer"),
    await readyMadeTitle(api, "my-open-issues"),
    await readyMadeTitle(api, "due-soon"),
  ];
  await page.goto(paths.home);
  // *Home* starts with the ready-made boxes of the blocks it showed before.
  await expect(boxOnScreen(page, open)).toBeVisible();
  await expect(boxOnScreen(page, due)).toBeVisible();
  await expect(boxOnScreen(page, timer)).toBeVisible();
  await page.getByRole("button", { name: t("dashboards.customize"), exact: true }).click();
  await expect(page.getByText(t("dashboards.changing-home"), { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: t("dashboards.box-menu", { title: due }), exact: true })
    .click();
  await page.getByRole("button", { name: t("dashboards.remove"), exact: true }).click();
  await expect(boxOnScreen(page, due)).toHaveCount(0);
  await expect
    .poll(async () =>
      dataOf(await api.client.GET("/api/v1/dashboards/home")).boxes.map((box) => box.title),
    )
    .toEqual([timer, open]);
  await page.getByRole("button", { name: t("dashboards.done"), exact: true }).click();
  await expect(
    page.getByRole("button", { name: t("dashboards.customize"), exact: true }),
  ).toBeVisible();
});
