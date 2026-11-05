import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes, inFourChosenThemesAsWindow } from "./support/four-themes.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { chipOf, growWorkDayLine, idleScene, scene, waitForParts } from "./support/time-scene.ts";

// The sidebar in a window 600 px high, with the timer and the work day and its menu of the group
// (M5-F150), in the four themes: a spec for a computer. Only taken when `BB_SCREENSHOT_DIR` names the
// folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

const MILESTONE = "M6/time";

test("the sidebar in a window 600 px high", async ({ page, server }) => {
  const { admin, issue } = await scene(server);
  await page.setViewportSize({ width: 1280, height: 600 });
  await signIn(page, ada);
  await inFourChosenThemes(page, "sidebar-600", MILESTONE, paths.appearance, async () => {
    await page.goto("/");
    await waitForParts(page, issue.key);
  });
  await admin.dispose();
});

test("the folded group of the sidebar, opened, in a window 600 px high", async ({
  page,
  server,
}) => {
  const { admin, issue } = await scene(server);
  await page.setViewportSize({ width: 1280, height: 600 });
  await signIn(page, ada);
  await inFourChosenThemesAsWindow(
    page,
    "sidebar-600-open",
    MILESTONE,
    paths.appearance,
    async () => {
      await page.goto("/");
      await waitForParts(page, issue.key);
      await page.getByRole("button", { name: t("shell.group-fold"), exact: true }).click();
      await expect(
        page.getByRole("link", { name: t("nav.my-settings"), exact: true }),
      ).toBeVisible();
    },
  );
  await admin.dispose();
});

test("a notice in a window 600 px high", async ({ page, server }) => {
  const { issue, reset, start, dispose } = await idleScene(server);
  await page.setViewportSize({ width: 1280, height: 600 });
  await signIn(page, ada);
  await inFourChosenThemesAsWindow(page, "notice-600", MILESTONE, paths.appearance, async () => {
    await reset();
    await start();
    await page.goto("/");
    await chipOf(page, issue.key).click();
    await page.getByRole("button", { name: t("time.timer.stop"), exact: true }).click();
    await page.getByRole("button", { name: t("time.timer.stop-timer"), exact: true }).click();
    await expect(
      page.getByText(t("time.timer.too-short", { issue: issue.key }), { exact: true }),
    ).toBeVisible();
  });
  await dispose();
});

test("a notice on an issue page in a window 600 px high", async ({ page, server }) => {
  const { issue, reset, start, dispose } = await idleScene(server);
  await page.setViewportSize({ width: 1280, height: 600 });
  await signIn(page, ada);
  await inFourChosenThemesAsWindow(
    page,
    "notice-600-issue",
    MILESTONE,
    paths.appearance,
    async () => {
      await reset();
      await start();
      await openIssueWithDetails(page, issue.key);
      await page.getByRole("button", { name: t("time.timer.bar-stop"), exact: true }).click();
      await page.getByRole("button", { name: t("time.timer.stop-timer"), exact: true }).click();
      await expect(
        page.getByText(t("time.timer.too-short", { issue: issue.key }), { exact: true }),
      ).toBeVisible();
    },
  );
  await dispose();
});

test("the sidebar of a tall window of a person without a saved search", async ({
  page,
  server,
}) => {
  const { dispose } = await idleScene(server);
  await signIn(page, ada);
  await inFourChosenThemes(
    page,
    "sidebar-tall-no-saved-searches",
    MILESTONE,
    paths.appearance,
    async () => {
      await page.goto("/");
      await expect(page.getByRole("region", { name: t("search.saved-heading") })).toBeVisible();
      await expect(page.getByText(t("search.saved-none"), { exact: true })).toBeVisible();
    },
  );
  await dispose();
});

test("the line of the work day three lines high, in a window 600 px high with the group open", async ({
  page,
  server,
}) => {
  const { admin, issue } = await scene(server);
  await page.setViewportSize({ width: 1280, height: 600 });
  await signIn(page, ada);
  await inFourChosenThemesAsWindow(
    page,
    "sidebar-600-long-line",
    MILESTONE,
    paths.appearance,
    async () => {
      await page.goto("/");
      await waitForParts(page, issue.key);
      await growWorkDayLine(page, 2);
      await page.getByRole("button", { name: t("shell.group-fold"), exact: true }).click();
      await expect(
        page.getByRole("link", { name: t("nav.my-settings"), exact: true }),
      ).toBeVisible();
    },
  );
  await admin.dispose();
});
