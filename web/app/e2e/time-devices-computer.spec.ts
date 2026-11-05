import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { viewports } from "../../tooling/viewports.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { chipOf } from "./support/time-scene.ts";
import { clearTimer, timerRunsOn } from "./support/time.ts";

// One timer for every device (TIME-01): it is started on a phone and shows on a computer. The test
// opens a phone and a computer itself, so it is run once, on a computer.

test.use({ preparation: { run: prepareWeb } });

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
}) => {
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
  await expect(chipOf(computer, issue.key)).toHaveCount(0);
  await openIssueWithDetails(phone, issue.key);
  await phone.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
  // An open issue on a phone has no bar of the app; its own bar says that the timer runs here.
  await expect(
    phone.getByRole("button", {
      name: new RegExp(`^${t("time.timer.stop-on", { issue: issue.key })}`, "u"),
    }),
  ).toBeVisible();
  // The computer gets the focus again and shows the same timer, counting from the server's start.
  await focusAgain(computer);
  await expect(chipOf(computer, issue.key)).toBeVisible();
  expect(await timerRunsOn(admin)).toBe(issue.key);
  await clearTimer(admin);
  await admin.dispose();
});
