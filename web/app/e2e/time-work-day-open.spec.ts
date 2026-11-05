import { t } from "@bugaboo/messages";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { dateDaysAgo, endWorkDayIfRunning, leaveDayOpen, workDayRuns } from "./support/work-day.ts";

// A work day of an earlier date that was never ended (TIME-08). A file of its own: such a day runs
// until now, so it cannot stand beside the days that other tests make today.

test.use({ preparation: { run: prepareWeb } });

test("time_08 a work day left open asks when it ended", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  await endWorkDayIfRunning(admin);
  const date = dateDaysAgo(2);
  await leaveDayOpen(admin, date);
  await signIn(page, ada);
  await page.goto("/");
  await page
    .getByRole("button", { name: t("time.work-day.left-open-button"), exact: true })
    .click();
  const question = page.getByRole("alertdialog");
  await expect(question).toBeVisible();
  await expectGoodInEveryLook(page);
  await expect(question).toContainText("?");
  await question.getByLabel(t("time.work-day.open-time"), { exact: true }).fill("17:30");
  await question.getByLabel(t("time.work-day.open-reason"), { exact: true }).fill("I forgot");
  await question.getByRole("button", { name: t("time.work-day.open-save"), exact: true }).click();
  await expect(question).toHaveCount(0);
  await expect(page.getByRole("button", { name: t("time.work-day.left-open-button") })).toHaveCount(
    0,
  );
  expect(await workDayRuns(admin)).toBe(false);
  await admin.dispose();
});
