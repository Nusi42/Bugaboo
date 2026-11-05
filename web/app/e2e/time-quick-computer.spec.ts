import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { ask, result } from "./support/quick-search.ts";
import { clearTimer, startedMinutesAgo, timerRunsOn } from "./support/time.ts";

// A spec for a computer: keys. *Start timer* and *Stop timer* in Quick search (CFG-08) and the key `t` on an open issue (UX-05):
// the keyboard alone starts and stops the timer and saves the time.

test.use({ preparation: { run: prepareWeb } });

/** The play button of the issue page: it is there once the page knows the person may book time. */
function startButton(page: Page) {
  return page.getByRole("button", { name: t("time.timer.start"), exact: true });
}

/** The window of *Stop timer* on this issue. */
function stopWindow(page: Page, issue: string) {
  return page.getByRole("dialog", { name: t("time.stop.title", { issue }) });
}

/** The question that a timer under a minute gets: nothing would be saved. */
function stopQuestion(page: Page, issue: string) {
  return page.getByRole("alertdialog", { name: t("time.stop.title", { issue }) });
}

test("cfg_08 quick search starts and stops the timer", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Printer jams");
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await expect(startButton(page)).toBeVisible();
  await ask(page, t("time.timer.start"));
  await result(page, t("time.timer.start")).first().click();
  await expect.poll(() => timerRunsOn(admin)).toBe(issue.key);
  // Starting a timer before the work day asks to start the day too: answer it, or it covers the page.
  const offer = page.getByRole("alertdialog", { name: t("time.work-day.too-title") });
  await offer.getByRole("button", { name: t("time.work-day.too-no"), exact: true }).click();
  await expect(offer).toHaveCount(0);
  // Start timer on the issue the timer runs on is asking to stop it.
  await ask(page, t("time.timer.start"));
  await result(page, t("time.timer.start")).first().click();
  await expect(stopQuestion(page, issue.key)).toBeVisible();
  await page.keyboard.press("Escape");
  await ask(page, t("time.timer.stop-timer"));
  await result(page, t("time.timer.stop-timer")).first().click();
  await expect(stopQuestion(page, issue.key)).toBeVisible();
  await clearTimer(admin);
  await admin.dispose();
});

test("ux_05 the keyboard alone starts and stops the timer and saves the time", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Paper runs out");
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await expect(startButton(page)).toBeVisible();
  await page.keyboard.press("t");
  await expect.poll(() => timerRunsOn(admin)).toBe(issue.key);
  await startedMinutesAgo(admin, 5);
  // The page looks at the timer again, with its new start.
  await page.reload();
  await expect(page.getByRole("definition").filter({ hasText: "0:05" })).toBeVisible();
  await page.keyboard.press("t");
  const window = stopWindow(page, issue.key);
  await expect(window).toBeVisible();
  await window.getByRole("button", { name: t("time.stop.save"), exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(window).toHaveCount(0);
  expect(await timerRunsOn(admin)).toBeUndefined();
  await admin.dispose();
});
