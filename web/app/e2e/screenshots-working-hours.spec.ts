import { t } from "@bugaboo/messages";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { ada, bob, cleo } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemesOf } from "./support/choice-pictures.ts";
import { inFourChosenThemes, inLightAndDarkChosen } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { prepareWorkingHoursPeople } from "./support/working-hours-people-scene.ts";

// *Organization settings › Working hours* and the time defaults of a project in the four themes, for the
// maintainer's review of M6. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWorkingHoursPeople } });

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
  // Only the part of the page about the defaults: the page also shows settings that only the test
  // server has (M6-F174).
  const defaults = () =>
    page.getByRole("region", { name: t("fields.time-defaults-project"), exact: true });
  await inFourThemesOf(
    page,
    defaults,
    { name: "project-defaults", milestone: MILESTONE },
    async () => {
      await page.goto(addressInProject(paths.projectSettings, "WEB"));
      await expect(defaults()).toBeVisible();
    },
  );
  await inFourThemesOf(
    page,
    defaults,
    { name: "sub-project-defaults", milestone: MILESTONE },
    async () => {
      await page.goto(addressInProject(paths.projectSettings, "APP"));
      await expect(
        page.getByText(t("fields.time-default-source", { project: "Website" })),
      ).toBeVisible();
    },
  );
});

test("the card of who sees working hours for somebody who may not read the roles", async ({
  page,
}) => {
  await signIn(page, bob);
  await inLightAndDarkChosen(page, "not-allowed", MILESTONE, paths.appearance, async () => {
    await page.goto(paths.workingHours);
    await expect(
      page.getByRole("region", { name: t("working-hours.who-title"), exact: true }),
    ).toContainText(t("permission.manage-roles"));
  });
});

test("the default of a project above that the project lead cannot see", async ({ page }) => {
  await signIn(page, cleo);
  await inLightAndDarkChosen(page, "hidden-source", MILESTONE, paths.appearance, async () => {
    await page.goto(addressInProject(paths.projectSettings, "APP"));
    await expect(page.getByText(t("fields.time-default-hidden-source"))).toBeVisible();
  });
});
