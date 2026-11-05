import { t } from "@bugaboo/messages";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { clearTimer, startTimerOn } from "./support/time.ts";
import { endWorkDayIfRunning } from "./support/work-day.ts";

// The sidebar with the timer and the work day fits a window 600 px high (M5-F150, UX §3.1).

test.use({ preparation: { run: prepareWeb } });

const WINDOW_HEIGHT = 600;

test("ux_01 the sidebar fits a window 600 px high", async ({ page, server, hasTouch }) => {
  test.skip(hasTouch, "The sidebar is the sidebar of a computer.");
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Printer jams");
  await endWorkDayIfRunning(admin);
  await startTimerOn(admin, issue);
  await page.setViewportSize({ width: 1280, height: WINDOW_HEIGHT });
  await signIn(page, ada);
  await page.goto("/");
  // The timer and the work day are there, with the buttons of a day that has not started.
  await expect(
    page.getByRole("button", {
      name: new RegExp(`^${t("time.timer.running-on")} ${issue.key}`, "u"),
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: t("time.work-day.start"), exact: true }),
  ).toBeVisible();
  const menu = page.getByRole("navigation", { name: t("shell.menu") });
  const sizes = await menu.evaluate((nav) => ({
    content: nav.scrollHeight,
    shown: nav.clientHeight,
  }));
  // Everything fits without scrolling inside the sidebar.
  expect(sizes.content).toBeLessThanOrEqual(sizes.shown);
  const signOut = page.getByRole("button", { name: t("nav.sign-out"), exact: true });
  // The last button is inside the window, not below its edge.
  const bottom = await signOut.evaluate((button) => button.getBoundingClientRect().bottom);
  expect(bottom).toBeLessThanOrEqual(WINDOW_HEIGHT);
  await expectGoodInEveryLook(page);
  await clearTimer(admin);
  await admin.dispose();
});
