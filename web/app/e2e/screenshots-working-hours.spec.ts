import { t } from "@bugaboo/messages";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { prepareWorkingHours } from "./support/working-hours-scene.ts";

// *Organization settings › Working hours* and the time defaults of a project in the four themes, for the
// maintainer's review of M6. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWorkingHours } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M6/working-hours";

test("the page of the working hours of the organization", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "working-hours", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.workingHours);
    await expect(
      page.getByRole("region", { name: t("working-hours.who-title"), exact: true }),
    ).toContainText("Payroll");
    await expect(page.getByText(/work days are older/u)).toBeVisible();
  });
});

test("the time defaults in the settings of a project and of its sub-project", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "project-defaults", MILESTONE, paths.appearance, async () => {
    await page.goto(addressInProject(paths.projectSettings, "WEB"));
    await expect(
      page.getByRole("region", { name: t("fields.time-defaults-project"), exact: true }),
    ).toBeVisible();
  });
  await inFourChosenThemes(page, "sub-project-defaults", MILESTONE, paths.appearance, async () => {
    await page.goto(addressInProject(paths.projectSettings, "APP"));
    await expect(page.getByText(t("fields.time-default-from-above"))).toBeVisible();
  });
});
