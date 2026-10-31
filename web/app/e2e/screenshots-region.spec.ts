import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { chooseTheme, themes } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder, shotWholeAs } from "./support/screenshots.ts";
import { inOrder } from "./support/sequence.ts";

// The screens whose dates and times of day changed with *Time of day* (CFG-04, UX-07), in the four
// themes, for the review of the pictures. They are only taken when `BB_SCREENSHOT_DIR` names the
// folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-region.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
const MILESTONE = "M6/region";

test.use({ preparation: { run: prepareWeb } });

/** Takes the open screen in the four themes. */
async function inFourThemes(page: Page, name: string): Promise<void> {
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shotWholeAs(page, `${name}-${test.info().project.name}`, MILESTONE, theme.id);
    }),
  );
}

test("the region page with its preview of a date and a time of day", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.region);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.region") })).toBeVisible();
  await inFourThemes(page, "region");
  await page
    .getByRole("radio", { name: t("setting.region.date-format.day-month-year"), exact: true })
    .click();
  await page
    .getByRole("radio", { name: t("setting.region.time-of-day.twelve-hours"), exact: true })
    .click();
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await inFourThemes(page, "region-day-first-12-hours");
  await page.goto(paths.sessions);
  await inFourThemes(page, "sessions-day-first-12-hours");
});
