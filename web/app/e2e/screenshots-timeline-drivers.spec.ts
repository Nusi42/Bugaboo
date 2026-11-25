import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { prepareDrivers } from "./support/timeline-drivers.ts";
import { warningsButton } from "./support/timeline-people.ts";

// Deadline drivers and releases on the timeline in the four themes, on a computer and on a phone
// (PLAN-05, ISS-08, UX §4.7): the outlined chain with its card and the warnings, and the rows
// grouped by release. Only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareDrivers } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M10/timeline-drivers";

/** The address of the search of WEB as a timeline with deadline drivers on, grouped by `group`. */
function address(group: string): string {
  return `/search?s=${encodeURIComponent("project is WEB")}&show=timeline&group=${encodeURIComponent(group)}&drivers=1`;
}

/** A bar outlined as a deadline driver on a computer, or the word of one on a phone. */
const DRIVER = '[data-state~="driver"], [data-part="timeline-phone.driver"]';

test("deadline drivers grouped by goal and by release, with the warnings", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(address("parent"));
  await expect(page.locator(DRIVER).first()).toBeVisible();
  await inFourThemes(page, "drivers-by-goal", MILESTONE);
  await warningsButton(page).click();
  await inFourThemes(page, "drivers-warnings", MILESTONE);
  await page.keyboard.press("Escape");
  await page.goto(address("planned for release"));
  await expect(page.locator(DRIVER).first()).toBeVisible();
  await inFourThemes(page, "drivers-by-release", MILESTONE);
});
