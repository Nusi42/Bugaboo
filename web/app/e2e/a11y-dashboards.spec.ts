import { t } from "@bugaboo/messages";

import { filled, paths, routeParam } from "../src/shared/paths.ts";
import { ada, signInThroughApi } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { readyMadeBox } from "./support/dashboard-issues.ts";
import { createDashboard, showAsTable } from "./support/dashboards.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The accessibility suite for dashboards (TEST-A11Y, CFG-03): a dashboard with a box of every kind, the same in
// *Change* with *Add box* open, and a chart shown as its table, in the four themes on a computer and a phone.

test.use({ preparation: { run: prepareWeb } });

/** A dashboard with a ready-made box of every kind, side by side as the ready-made boxes start; its address. */
async function everyKind(server: Parameters<typeof signInThroughApi>[0]): Promise<string> {
  const admin = await signInThroughApi(server, ada);
  const kinds = await Promise.all(
    (
      [
        "open-bugs-by-priority",
        "my-open-issues",
        "time-on-this-search",
        "my-time-this-week",
        "recent-changes-in-this-project",
        "my-timer",
        "small-timeline-of-a-search",
      ] as const
    ).map((which) => readyMadeBox(admin, which)),
  );
  const made = await createDashboard(
    admin,
    "Every kind",
    kinds.map((box, at) => Object.assign(box, { column: (at % 2) * 6, row: at * 8 })),
  );
  await admin.dispose();
  return filled(paths.dashboard, routeParam.dashboard, made.id);
}

test("a11y a dashboard with a box of every kind, and its charts as tables", async ({
  page,
  server,
}) => {
  const address = await everyKind(server);
  await signIn(page, ada);
  await page.goto(address);
  await expect(page.getByRole("figure").first()).toBeVisible();
  await expectGoodInEveryLook(page);
  await showAsTable(page.getByRole("figure").first());
  await expectGoodInEveryLook(page);
});

test("a11y a dashboard in Change, with Add box open", async ({ page, server }) => {
  const address = await everyKind(server);
  await signIn(page, ada);
  await page.goto(address);
  await page.getByRole("button", { name: t("dashboards.change"), exact: true }).click();
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("dashboards.add-box"), exact: true }).click();
  await expect(page.getByRole("dialog", { name: t("dashboards.add-box") })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y Home with its boxes, in Customize", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.home);
  await page.getByRole("button", { name: t("dashboards.customize"), exact: true }).click();
  await expect(page.getByText(t("dashboards.changing-home"), { exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
});
