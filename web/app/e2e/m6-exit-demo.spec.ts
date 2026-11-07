import { STEP_TIMEOUT_MS } from "./support/demo-m6-common.ts";
import { anaStartsHerDayAndStopsATimer, anaTakesABreakAndEndsTheDay } from "./support/demo-m6.ts";
import {
  anaChangesYesterdayWithAReason,
  aLongDayShowsItsWarning,
} from "./support/demo-m6-hours.ts";
import {
  aTimeEntryInTheHistoryOfAnotherReader,
  aTimeEntryInTheHistoryOfItsBooker,
  theWindowsOfTime,
  thePermissionNames,
} from "./support/demo-m6-more.ts";
import { benCannotSeeThem, hanaSeesThem } from "./support/demo-m6-people.ts";
import { test } from "./support/fixtures.ts";

// The exit demo of M6 (docs/04-milestones.md) on the sample organization, as the maintainer can
// repeat it by hand with `cargo xtask dev --sample`: Ana starts her work day, runs a timer and stops
// the timer (the window shows *Activity* and *Cost center*), takes a break and ends the day; the timesheet
// warns of a long day; Ana changes yesterday's end and gives a reason; Ben cannot see the working
// hours of Ana, Hana sees them, and with *Totals only* on she sees only the total of the week. Every
// step is good in the four themes on a phone and on a computer, and with `BB_SCREENSHOT_DIR` set the
// same run takes the pictures of the review page
// (`<folder>/M6/demo/<step>-<phone|computer>-<theme>.png`). The steps are in `support/demo-m6.ts`.

test.use({ sample: true });
test.describe.configure({ mode: "serial" });

test("1. Ana starts her work day, runs a timer and stops it", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await anaStartsHerDayAndStopsATimer(page, server);
});

test("2. Ana takes a break and ends her day", async ({ page }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await anaTakesABreakAndEndsTheDay(page);
});

test("3. The timesheet shows the warning of a long day", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await aLongDayShowsItsWarning(page, server);
});

test("4. Ana changes yesterday's end and gives a reason", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await anaChangesYesterdayWithAReason(page, server);
});

test("5. Ben cannot see the working hours of Ana", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await benCannotSeeThem(page, server);
});

test("6. Hana sees them, and with Totals only only the total of the week", async ({
  page,
  server,
}) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await hanaSeesThem(page, server);
});

test("7. Add time and Change start time write dates in the person's Region", async ({
  page,
  server,
}) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await theWindowsOfTime(page, server);
});

test("8. A time entry in the history, for the person who booked it", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await aTimeEntryInTheHistoryOfItsBooker(page, server);
});

test("9. The same time entry in the history, for another reader", async ({ page }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await aTimeEntryInTheHistoryOfAnotherReader(page);
});

test("10. The permissions of working hours have their glossary names", async ({ page }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await thePermissionNames(page);
});
