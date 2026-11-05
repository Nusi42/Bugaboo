import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { viewports } from "../../tooling/viewports.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import {
  choiceNamed,
  clearTimer,
  entriesBookedOn,
  setTimeDefault,
  startedMinutesAgo,
  starterField,
  startTimerOn,
  timerRunsOn,
} from "./support/time.ts";

// The timer on screen (TIME-01): one timer per person that every device shows, *Stop timer* with the
// fields of the entry filled in from the issue, and *Switch issue* that keeps one timer running.

test.use({ preparation: { run: prepareWeb } });

/** The button that is the running timer, on the issue with this ID. */
function chipOn(page: Page, issue: string) {
  return page
    .getByRole("button", { name: t("time.timer.running", { issue }), exact: true })
    .first();
}

/** Says to the page that it got the focus, as coming back to its window does. */
async function focusAgain(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.dispatchEvent(new Event("visibilitychange"));
    window.dispatchEvent(new Event("focus"));
  });
}

test("time_01 the timer started on the phone shows on the computer", async ({
  browser,
  server,
  hasTouch,
}) => {
  test.skip(hasTouch, "The test opens a phone and a computer itself.");
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Login fails after password reset");
  const options = { baseURL: server.url, ignoreHTTPSErrors: true };
  const computer = await (
    await browser.newContext({ ...options, viewport: viewports.computer })
  ).newPage();
  const phone = await (
    await browser.newContext({ ...options, viewport: viewports.phone, hasTouch: true })
  ).newPage();
  await signIn(computer, ada);
  await signIn(phone, ada);
  await expect(chipOn(computer, issue.key)).toHaveCount(0);
  await openIssueWithDetails(phone, issue.key);
  await phone.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
  // An open issue on a phone has no bar of the app; its own bar says that the timer runs here.
  await expect(
    phone.getByRole("button", { name: t("time.timer.stop-timer"), exact: true }).first(),
  ).toBeVisible();
  // The computer gets the focus again and shows the same timer, counting from the server's start.
  await focusAgain(computer);
  await expect(chipOn(computer, issue.key)).toBeVisible();
  expect(await timerRunsOn(admin)).toBe(issue.key);
  await clearTimer(admin);
  await admin.dispose();
});

test("time_01 stop timer shows activity and cost center from the issue", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Login fails after password reset");
  const activity = await starterField(admin, "activity");
  const costCenter = await starterField(admin, "cost-center");
  const testing = await choiceNamed(admin, activity, "Testing");
  const product = await choiceNamed(admin, costCenter, "4100 Product development");
  await setTimeDefault(admin, issue, activity, testing);
  await setTimeDefault(admin, issue, costCenter, product);
  await startTimerOn(admin, issue);
  await startedMinutesAgo(admin, 42);
  await signIn(page, ada);
  await page.goto("/");
  await chipOn(page, issue.key).click();
  await page.getByRole("button", { name: t("time.timer.stop"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("time.stop.title", { issue: issue.key }) });
  await expect(window).toBeVisible();
  await expect(window).toContainText(
    t("time.stop.worked", { length: "42m", title: "Login fails after password reset" }),
  );
  // The boxes start with the defaults of the issue, which the server worked out.
  await expect(window.getByLabel("Activity", { exact: true })).toHaveValue(testing);
  await expect(window.getByLabel("Cost center", { exact: true })).toHaveValue(product);
  await window.getByLabel(t("time.entry.note"), { exact: true }).fill("Found the cause");
  await window.getByRole("button", { name: t("time.stop.save"), exact: true }).click();
  await expect(window).toHaveCount(0);
  await expect(chipOn(page, issue.key)).toHaveCount(0);
  await expect(
    page.getByText(t("time.timer.saved", { length: "42m", issue: issue.key }), { exact: true }),
  ).toBeVisible();
  const [saved] = await entriesBookedOn(admin, issue);
  expect(saved?.minutes).toBe(42);
  expect(saved?.note).toBe("Found the cause");
  expect(saved?.fields[activity]).toBe(testing);
  await admin.dispose();
});
