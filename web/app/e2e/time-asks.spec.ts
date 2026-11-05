import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { chipOf, idleScene, scene } from "./support/time-scene.ts";

// Every window and question of the timer and the work day, open, is good in every look (TEST-A11Y,
// UX §4.9): *Stop timer*, the timer menu, *Change start time*, *Switch issue*, *Throw away*, and the
// two questions of the work day. *When did your work day … end?* is in `time-work-day-open.spec.ts`.

test.use({ preparation: { run: prepareWeb } });

/** Opens the menu of the running timer and chooses one of its entries. */
async function chooseFromTimer(page: Page, issue: string, entry: string): Promise<void> {
  await chipOf(page, issue).click();
  await page.getByRole("button", { name: entry, exact: true }).click();
}

test("time_01 the windows of the timer are good in every look", async ({ page, server }) => {
  const { admin, issue } = await scene(server);
  await signIn(page, ada);
  await page.goto("/");
  await chipOf(page, issue.key).click();
  await expect(
    page.getByRole("button", { name: t("time.timer.throw-away"), exact: true }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await chooseFromTimer(page, issue.key, t("time.timer.stop"));
  await expect(page.getByRole("dialog")).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await chooseFromTimer(page, issue.key, t("time.timer.change-start"));
  await expect(page.getByRole("dialog", { name: t("time.start-time.title") })).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await chooseFromTimer(page, issue.key, t("time.timer.switch"));
  await expect(page.getByRole("dialog", { name: t("time.switch.title") })).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await chooseFromTimer(page, issue.key, t("time.timer.throw-away"));
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await expectGoodInEveryLook(page);
  await admin.dispose();
});

test("time_08 the questions of the work day are good in every look", async ({ page, server }) => {
  const { issue, workDayStarted, dispose } = await idleScene(server);
  await signIn(page, ada);
  await openIssueWithDetails(page, issue.key);
  await page.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
  const asked = page.getByRole("alertdialog", { name: t("time.work-day.too-title") });
  await expect(asked).toBeVisible();
  await expectGoodInEveryLook(page);
  await asked.getByRole("button", { name: t("time.work-day.too-yes"), exact: true }).click();
  // The work day ends while the timer runs: *Stop the timer on … too?*.
  await expect.poll(workDayStarted).toBe(true);
  await page.goto("/");
  await expect(page.getByText(/^Working since/u)).toBeVisible();
  await page.waitForTimeout(1100);
  await page.getByRole("button", { name: t("time.work-day.end"), exact: true }).click();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await expectGoodInEveryLook(page);
  await dispose();
});

test("time_01 a stop under one minute says that nothing will be saved", async ({
  page,
  server,
}) => {
  const { issue, start, dispose } = await idleScene(server);
  await start();
  await signIn(page, ada);
  await page.goto("/");
  await chipOf(page, issue.key).click();
  await page.getByRole("button", { name: t("time.timer.stop"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("time.stop.title", { issue: issue.key }) });
  await expect(window).toContainText(t("time.stop.nothing", { issue: issue.key }));
  await window.getByRole("button", { name: t("time.stop.save"), exact: true }).click();
  await expect(
    page.getByText(t("time.timer.too-short", { issue: issue.key }), { exact: true }).first(),
  ).toBeVisible();
  await dispose();
});

test("time_01 the issue page shows the running timer in a row of its own", async ({
  page,
  server,
}) => {
  const { admin, issue } = await scene(server);
  await signIn(page, ada);
  await openIssueWithDetails(page, issue.key);
  const row = page.getByRole("term").filter({ hasText: t("time.timer.row") });
  await expect(row).toBeVisible();
  // The clock is in hours and minutes, and *Stop* opens *Stop timer*.
  await expect(page.getByText("0:42", { exact: true }).first()).toBeVisible();
  await page
    .getByRole("button", { name: t("time.timer.bar-stop"), exact: true })
    .first()
    .click();
  await expect(
    page.getByRole("dialog", { name: t("time.stop.title", { issue: issue.key }) }),
  ).toBeVisible();
  await admin.dispose();
});
