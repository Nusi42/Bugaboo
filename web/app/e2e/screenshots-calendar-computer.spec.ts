import { ada } from "./support/accounts.ts";
import { chooseChangeDay, dayOf, itemOf, pressOf, weekDays } from "./support/calendar-places.ts";
import { openCalendar, openProjectCalendar, prepareCalendar, scene } from "./support/calendar.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The calendar on a computer in the four themes (UX §4.16): a month, a week with its marks, the list of a
// crowded day, an issue picked up with the spacebar, and the window *Change due date …*. Only taken when
// `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareCalendar } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M10/calendar";

test("the calendar as a month and as a week, with its marks", async ({ page }) => {
  await signIn(page, ada);
  await openCalendar(page);
  await expect(itemOf(page, scene.crowdFirst)).toBeVisible();
  await inFourThemes(page, "month", MILESTONE);
  await openProjectCalendar(page);
  await expect(itemOf(page, scene.dragged)).toBeVisible();
  await inFourThemes(page, "week", MILESTONE);
});

test("the list of a crowded day, an issue picked up, and Change due date", async ({ page }) => {
  const { crowded } = weekDays();
  await signIn(page, ada);
  await openCalendar(page);
  await dayOf(page, crowded)
    .getByRole("button", { name: /^\+\d+ more on /u })
    .click();
  await inFourThemesAsWindow(page, "day-list", MILESTONE);
  await page.keyboard.press("Escape");
  await openProjectCalendar(page);
  const press = pressOf(page, scene.dragged);
  await press.focus();
  await page.keyboard.press("Space");
  await page.keyboard.press(weekDays().arrow);
  await inFourThemes(page, "picked-up", MILESTONE);
  await page.keyboard.press("Escape");
  await chooseChangeDay(page, scene.typed);
  await inFourThemesAsWindow(page, "change-due-date", MILESTONE);
});
