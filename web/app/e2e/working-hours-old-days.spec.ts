import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { createGroup, giveGroupRole, signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { createDays, daysAgo, press } from "./support/working-hours-scene.ts";

// The work days past the keep period with *Delete them*, *Totals only* and who holds the permission to see
// working hours of others, on *Organization settings › Working hours* (TIME-10, TIME-12, ADR-0035, decisions 7 and 8).

test.use({ preparation: { run: prepareWeb } });

test("time_10 Delete them asks first and deletes only days older than the keep period", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const young = daysAgo(10);
  await createDays(admin, ["2020-03-02", "2020-03-03", young]);
  await signIn(page, ada);
  await page.goto(paths.workingHours);
  const card = page.getByRole("region", { name: t("old-work-days.title") });
  await expect(
    card.getByText(/^2 work days are older than the keep period of 2 years/u),
  ).toBeVisible();
  // Nothing is deleted by asking: the question comes first, and *Cancel* leaves everything.
  await card.getByRole("button", { name: t("old-work-days.delete-all"), exact: true }).click();
  const question = page.getByRole("alertdialog");
  await expect(question).toContainText(t("old-work-days.delete-all-text", { count: 2 }));
  await question.getByRole("button", { name: t("common.cancel"), exact: true }).click();
  await expect(card.getByText(/^2 work days are older/u)).toBeVisible();
  await card.getByRole("button", { name: t("old-work-days.delete-all"), exact: true }).click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("old-work-days.delete"), exact: true })
    .click();
  await expect(card.getByText(t("old-work-days.deleted", { count: 2 }))).toBeVisible();
  await expect(card.getByText(t("old-work-days.none"))).toBeVisible();
  // The day younger than the keep period is still there.
  const kept = dataOf(
    await admin.client.GET("/api/v1/me/work-days", {
      params: { query: { from: young, to: young } },
    }),
  );
  expect(kept.days).toHaveLength(1);
  await admin.dispose();
});

test("time_12 Totals only is a switch, and the page says who holds the permission to see working hours", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await signIn(page, ada);
  await page.goto(paths.workingHours);
  const who = page.getByRole("region", { name: t("working-hours.who-title") });
  await expect(who.getByText(t("working-hours.who-nobody"))).toBeVisible();
  await expect(who.getByRole("link", { name: t("nav.roles"), exact: true })).toHaveAttribute(
    "href",
    paths.roles,
  );
  const payroll = await createGroup(admin, "Payroll");
  await giveGroupRole(admin, payroll, "HR", null);
  await page.reload();
  await expect(who.getByRole("listitem")).toHaveText(["Payroll"]);
  const totals = t("setting.working-hours.totals-only.name");
  await press(page, totals, true);
  await admin.dispose();
  await expectGoodInEveryLook(page);
});
