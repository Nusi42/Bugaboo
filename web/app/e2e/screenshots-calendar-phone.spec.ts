import { ada } from "./support/accounts.ts";
import { prepareCalendar, scene } from "./support/calendar.ts";
import { chooseChangeDay, itemOf } from "./support/calendar-places.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The calendar on a phone in the four themes (UX §4.16): the list of the days to come and the window
// *Change due date …*. Only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareCalendar } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M10/calendar";

test("the phone list of the days to come and Change due date", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/p/WEB/issues?show=calendar");
  await expect(itemOf(page, scene.dragged)).toBeVisible();
  await inFourThemes(page, "days", MILESTONE);
  await chooseChangeDay(page, scene.dragged);
  await inFourThemesAsWindow(page, "change-due-date", MILESTONE);
});
