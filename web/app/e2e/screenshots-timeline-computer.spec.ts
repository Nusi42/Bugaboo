import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { chart, openPlan, planned, prepareTimeline, pressOf } from "./support/timeline.ts";

// The timeline on a computer in the four themes (UX §4.7): grouped by goal, a selected bar with its
// card, the windows of the typed ways, the zooms of days and quarters, and a narrow window where the
// side stands under the chart. They are only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareTimeline } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M8/timeline";

/** A computer window narrower than the timeline beside its side column. */
const narrow = { width: 1000, height: 900 };

test("the timeline grouped by goal, a selected bar and its windows", async ({ page }) => {
  await signIn(page, ada);
  await openPlan(page);
  await inFourThemes(page, "grouped-by-goal", MILESTONE);
  await pressOf(page, planned.waits).click();
  await expect(
    page.getByRole("region", { name: t("timeline.card", { key: planned.waits }) }),
  ).toBeVisible();
  await inFourThemes(page, "selected-bar", MILESTONE);
  await page
    .getByRole("button", { name: t("timeline.set-dates"), exact: true })
    .first()
    .click();
  await inFourThemesAsWindow(page, "set-dates", MILESTONE);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: t("timeline.add-dependency"), exact: true }).click();
  await inFourThemesAsWindow(page, "add-dependency", MILESTONE);
  await page.keyboard.press("Escape");
});

test("the zooms of days and quarters, and a narrow window", async ({ page }) => {
  await signIn(page, ada);
  await openPlan(page);
  const zoom = page.getByRole("group", { name: t("timeline.zoom") });
  await zoom.getByRole("radio", { name: t("timeline.zoom-day") }).check();
  await expect(chart(page)).toBeVisible();
  await inFourThemes(page, "zoom-days", MILESTONE);
  await zoom.getByRole("radio", { name: t("timeline.zoom-quarter") }).check();
  await inFourThemes(page, "zoom-quarters", MILESTONE);
  await zoom.getByRole("radio", { name: t("timeline.zoom-week") }).check();
  await page.setViewportSize(narrow);
  await expect(chart(page)).toBeVisible();
  await inFourThemes(page, "narrow-window", MILESTONE);
});
