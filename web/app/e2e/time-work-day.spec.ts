import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { clearTimer, startTimerOn, timerRunsOn } from "./support/time.ts";
import { endWorkDayIfRunning, workDayRuns } from "./support/work-day.ts";

// The work day on screen (TIME-08): the four buttons and the line of today's hours, the question
// "Start your work day too?" when a timer starts before the work day, and "Stop the timer on WEB-142
// too?" when the work day ends while a timer runs.

test.use({ preparation: { run: prepareWeb } });

/** A button of the work day by its words. */
function button(page: Page, key: Parameters<typeof t>[0]) {
  return page.getByRole("button", { name: t(key), exact: true });
}

/** A little over a second: the server takes one change of the work day a second from a person. */
const TAP_PAUSE_MS = 1100;

/** Presses a button of the work day after the pause the server asks for between two changes. */
async function tap(page: Page, key: Parameters<typeof t>[0]): Promise<void> {
  await page.waitForTimeout(TAP_PAUSE_MS);
  await button(page, key).click();
}

/** The line of today's hours, by its first words. */
function line(page: Page, words: string) {
  return page.getByText(new RegExp(`^${words}`, "u"));
}

test("time_08 start break, end break and end work day from the sidebar", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await endWorkDayIfRunning(admin);
  await signIn(page, ada);
  await page.goto("/");
  await tap(page, "time.work-day.start");
  await expect(line(page, "Working since")).toBeVisible();
  await expectGoodInEveryLook(page);
  await tap(page, "time.work-day.break-start");
  await expect(line(page, "On break since")).toBeVisible();
  await expect(button(page, "time.work-day.break-end")).toBeVisible();
  await tap(page, "time.work-day.break-end");
  await expect(line(page, "Working since")).toBeVisible();
  await tap(page, "time.work-day.end");
  await expect(line(page, "Worked ")).toBeVisible();
  await expect(button(page, "time.work-day.start")).toBeVisible();
  expect(await workDayRuns(admin)).toBe(false);
  await admin.dispose();
});

test("time_08 ending the work day while a timer runs asks to stop the timer too", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Printer jams");
  await endWorkDayIfRunning(admin);
  await startTimerOn(admin, issue);
  await signIn(page, ada);
  await page.goto("/");
  await tap(page, "time.work-day.start");
  await expect(line(page, "Working since")).toBeVisible();
  // "Keep it running" ends the day only.
  await tap(page, "time.work-day.end");
  const question = page.getByRole("alertdialog", {
    name: t("time.work-day.timer-title", { issue: issue.key }),
  });
  await expect(question).toBeVisible();
  await question.getByRole("button", { name: t("time.work-day.timer-no"), exact: true }).click();
  await expect(line(page, "Worked ")).toBeVisible();
  expect(await timerRunsOn(admin)).toBe(issue.key);
  // "Stop the timer too" ends the day and opens Stop timer.
  await tap(page, "time.work-day.start");
  await expect(line(page, "Working since")).toBeVisible();
  await tap(page, "time.work-day.end");
  await question.getByRole("button", { name: t("time.work-day.timer-yes"), exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: t("time.stop.title", { issue: issue.key }) }),
  ).toBeVisible();
  expect(await workDayRuns(admin)).toBe(false);
  await clearTimer(admin);
  await admin.dispose();
});

test("time_08 starting a timer before the work day asks to start it too", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Paper runs out");
  await endWorkDayIfRunning(admin);
  await signIn(page, ada);
  await openIssueWithDetails(page, issue.key);
  await page.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
  const question = page.getByRole("alertdialog", { name: t("time.work-day.too-title") });
  await expect(question).toBeVisible();
  await expect(question).toContainText(t("time.work-day.too-text", { issue: issue.key }));
  await question.getByRole("button", { name: t("time.work-day.too-yes"), exact: true }).click();
  await expect(question).toHaveCount(0);
  // The work day starts after the question closes: the answer is waited for.
  await expect.poll(() => workDayRuns(admin)).toBe(true);
  expect(await timerRunsOn(admin)).toBe(issue.key);
  await clearTimer(admin);
  await admin.dispose();
});

test("time_08 not now is not asked again with the next timer of the day", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const first = await createIssue(admin, "WEB", "Toner is low");
  const second = await createIssue(admin, "WEB", "Tray is stuck");
  await endWorkDayIfRunning(admin);
  await signIn(page, ada);
  await openIssueWithDetails(page, first.key);
  await page.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
  const question = page.getByRole("alertdialog", { name: t("time.work-day.too-title") });
  await question.getByRole("button", { name: t("time.work-day.too-no"), exact: true }).click();
  await expect(question).toHaveCount(0);
  await page.waitForTimeout(TAP_PAUSE_MS);
  await openIssueWithDetails(page, second.key);
  await page.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
  // The issue page of a phone has no bar of the app: its own bar says the timer runs here.
  await expect(
    page.getByRole("button", { name: t("time.timer.stop-timer"), exact: true }).first(),
  ).toBeVisible();
  await expect(question).toHaveCount(0);
  expect(await workDayRuns(admin)).toBe(false);
  await clearTimer(admin);
  await admin.dispose();
});
