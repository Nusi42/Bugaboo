import { timeEntryMinutesLeast } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook, inEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { chipOf, growWorkDayLine, idleScene, scene, waitForParts } from "./support/time-scene.ts";

// The sidebar with the timer and the work day fits a window 600 px high, and its group at the bottom is
// a menu that rises from its button (M5-F150, UX §3.1). A spec for a computer: it is a sidebar.

test.use({ preparation: { run: prepareWeb } });

const WINDOW_HEIGHT = 600;
const SIDEBAR_WIDTH = 264;

/** Everything of the sidebar is in the window: it does not scroll, and *Sign out* is above the edge. */
async function expectSidebarFits(page: Page): Promise<void> {
  const menu = page.getByRole("navigation", { name: t("shell.menu") });
  const sizes = await menu.evaluate((nav) => ({
    content: nav.scrollHeight,
    shown: nav.clientHeight,
  }));
  expect(sizes.content).toBeLessThanOrEqual(sizes.shown);
  const signOut = page.getByRole("button", { name: t("nav.sign-out"), exact: true });
  const bottom = await signOut.evaluate((button) => button.getBoundingClientRect().bottom);
  expect(bottom).toBeLessThanOrEqual(WINDOW_HEIGHT);
}

test("ux_01 the sidebar fits a window 600 px high in every state of the timer and the work day", async ({
  page,
  server,
}) => {
  const { issue, start, dispose } = await idleScene(server);
  await start();
  await page.setViewportSize({ width: 1280, height: WINDOW_HEIGHT });
  await signIn(page, ada);
  await page.goto("/");
  await expect(chipOf(page, issue.key)).toBeVisible();
  // The timer runs and the day has not started.
  await expect(
    page.getByRole("button", { name: t("time.work-day.start"), exact: true }),
  ).toBeVisible();
  await expectSidebarFits(page);
  await expectGoodInEveryLook(page);
  // The day started.
  await page.getByRole("button", { name: t("time.work-day.start"), exact: true }).click();
  await expect(page.getByText(/^Working since/u)).toBeVisible();
  await expectSidebarFits(page);
  // A notice: the stop of a timer under a minute says that nothing was saved. It floats over the page.
  await chipOf(page, issue.key).click();
  await page.getByRole("button", { name: t("time.timer.stop"), exact: true }).click();
  await page.getByRole("button", { name: t("time.timer.stop-timer"), exact: true }).click();
  const notice = page.getByText(
    t("time.timer.too-short", { issue: issue.key, minutes: timeEntryMinutesLeast }),
    { exact: true },
  );
  await expect(notice).toBeVisible();
  await expectSidebarFits(page);
  const left = await notice.evaluate((element) => element.getBoundingClientRect().left);
  expect(left).toBeGreaterThanOrEqual(SIDEBAR_WIDTH);
  await expectGoodInEveryLook(page);
  await dispose();
});

test("ux_01 the menu of the group at the bottom rises over the page and gives the focus back", async ({
  page,
  server,
}) => {
  // The work day runs and the timer too: the highest the sidebar gets.
  const { admin, issue } = await scene(server);
  await page.setViewportSize({ width: 1280, height: WINDOW_HEIGHT });
  await signIn(page, ada);
  await page.goto("/");
  await waitForParts(page, issue.key);
  await inEveryLook(page, () => expectSidebarFits(page));
  // The line of today's hours may wrap to one more line than it has, as it does after hours of work: it
  // is made one line higher than it is (a style, so a new clock does not undo it).
  const heights = await growWorkDayLine(page, 1);
  expect(Number.isFinite(heights.after)).toBe(true);
  expect(heights.after).toBeGreaterThan(heights.before);
  await inEveryLook(page, () => expectSidebarFits(page));
  // The group is one button; its menu rises over the page, and the sidebar does not grow or scroll.
  const group = page.getByRole("button", { name: t("shell.group-fold"), exact: true });
  await group.click();
  const opened = page.getByRole("dialog", { name: t("shell.group-fold") });
  await expect(opened.getByRole("link", { name: t("nav.my-settings"), exact: true })).toBeVisible();
  await expectSidebarFits(page);
  // The focus is inside the menu.
  await expect(opened.and(page.locator(":focus-within"))).toHaveCount(1);
  await expectGoodInEveryLook(page);
  // Opened, the sidebar is as high as closed, in every theme, and *Sign out* stays in view.
  const height = () =>
    page
      .getByRole("navigation", { name: t("shell.menu") })
      .evaluate((nav) => nav.getBoundingClientRect().height);
  const openHeight = await height();
  await inEveryLook(page, async () => {
    await expectSidebarFits(page);
    expect(await height()).toBe(openHeight);
  });
  // Esc closes it and the focus goes back to the button.
  await page.keyboard.press("Escape");
  await expect(opened).toHaveCount(0);
  await expect(group).toBeFocused();
  await admin.dispose();
});

test("ux_01 the narrow rail names the timer and tells what it did beside the rail", async ({
  page,
  server,
}) => {
  const { issue, start, dispose } = await idleScene(server);
  await start();
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  // The issue page narrows the sidebar to icons; the timer is one button named by its words.
  const chip = chipOf(page, issue.key);
  await expect(chip).toBeVisible();
  await expect(chip).toHaveAccessibleName(
    new RegExp(`^${t("time.timer.running", { issue: issue.key })} [0-9]+:[0-9]{2}$`, "u"),
  );
  // What the timer tells is not lost for the narrow rail: it shows beside it, not over the Details.
  await page.getByRole("button", { name: t("time.timer.bar-stop"), exact: true }).click();
  await page.getByRole("button", { name: t("time.timer.stop-timer"), exact: true }).click();
  const notice = page.getByText(
    t("time.timer.too-short", { issue: issue.key, minutes: timeEntryMinutesLeast }),
    { exact: true },
  );
  await expect(notice).toBeVisible();
  const details = page.getByRole("term").filter({ hasText: t("issue-field.time-spent") });
  const right = await notice.evaluate((element) => element.getBoundingClientRect().right);
  const detailsLeft = await details.evaluate((element) => element.getBoundingClientRect().left);
  expect(right).toBeLessThan(detailsLeft);
  // Nothing of the page is drawn over it: the point in the middle of the notice belongs to the notice.
  const alert = page.getByRole("status").filter({ has: notice });
  const onTop = await alert.evaluate((element) => {
    const box = element.getBoundingClientRect();
    return element.contains(
      document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2),
    );
  });
  expect(onTop).toBe(true);
  await dispose();
});
