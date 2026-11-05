import { t } from "@bugaboo/messages";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { chipOf, idleScene, scene, waitForParts } from "./support/time-scene.ts";

// The screens of the timer and the work day (TIME-01, TIME-02, TIME-08) in the four themes, for the
// review of the screens. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

const MILESTONE = "M6/time";

test("the timer and the work day in the sidebar and on Home", async ({ page, server }) => {
  const { admin, issue } = await scene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "timer-and-work-day", MILESTONE, paths.appearance, async () => {
    await page.goto("/");
    await waitForParts(page, issue.key);
  });
  await admin.dispose();
});

test("the timer menu", async ({ page, server }) => {
  const { admin, issue } = await scene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "timer-menu", MILESTONE, paths.appearance, async () => {
    await page.goto("/");
    await waitForParts(page, issue.key);
    await chipOf(page, issue.key).click();
    await expect(
      page.getByRole("button", { name: t("time.timer.throw-away"), exact: true }),
    ).toBeVisible();
  });
  await admin.dispose();
});

test("stop timer", async ({ page, server }) => {
  const { admin, issue } = await scene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "stop-timer", MILESTONE, paths.appearance, async () => {
    await page.goto("/");
    await waitForParts(page, issue.key);
    await chipOf(page, issue.key).click();
    await page.getByRole("button", { name: t("time.timer.stop"), exact: true }).click();
    await expect(
      page.getByRole("dialog", { name: t("time.stop.title", { issue: issue.key }) }),
    ).toBeVisible();
  });
  await admin.dispose();
});

test("add time", async ({ page, server }) => {
  const { admin, issue } = await scene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "add-time", MILESTONE, paths.appearance, async () => {
    await openIssueWithDetails(page, issue.key);
    await page.getByRole("button", { name: t("time.add.button"), exact: true }).click();
    await expect(
      page.getByRole("dialog", { name: t("time.add.title", { issue: issue.key }) }),
    ).toBeVisible();
  });
  await admin.dispose();
});

test("the issue page with the timer", async ({ page, server }) => {
  const { admin, issue } = await scene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "issue-page", MILESTONE, paths.appearance, async () => {
    await page.goto(issueAddress(issue.key));
    await expect(
      page
        .getByRole("button", {
          name: new RegExp(`^(${t("time.timer.bar-stop")}|${t("time.timer.stop-timer")})$`, "u"),
        })
        .first(),
    ).toBeVisible();
  });
  await admin.dispose();
});

test("start your work day too", async ({ page, server }) => {
  const { issue, reset, dispose } = await idleScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(page, "start-work-day-too", MILESTONE, paths.appearance, async () => {
    await reset();
    await openIssueWithDetails(page, issue.key);
    await page.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
    await expect(
      page.getByRole("alertdialog", { name: t("time.work-day.too-title") }),
    ).toBeVisible();
  });
  await dispose();
});

test("the sidebar in a window 600 px high", async ({ page, server, hasTouch }) => {
  test.skip(hasTouch, "The sidebar is the sidebar of a computer.");
  const { admin, issue } = await scene(server);
  await page.setViewportSize({ width: 1280, height: 600 });
  await signIn(page, ada);
  await inFourChosenThemes(page, "sidebar-600", MILESTONE, paths.appearance, async () => {
    await page.goto("/");
    await waitForParts(page, issue.key);
  });
  await admin.dispose();
});
