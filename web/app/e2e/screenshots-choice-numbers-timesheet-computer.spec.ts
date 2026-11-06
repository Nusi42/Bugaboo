import { ada } from "./support/accounts.ts";
import { weekWithCostCenters } from "./support/cost-center-week.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { pictures } from "./support/timesheet-pictures.ts";
import { openWeek } from "./support/timesheet.ts";

// The timesheet with cost centers in the four themes, for the review of M6. Only taken when
// `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

test("the timesheet with two cost centers on one row", async ({ page, server }) => {
  const first = await weekWithCostCenters(server);
  await signIn(page, ada);
  await pictures(page, "timesheet-cost-centers", async () => {
    await openWeek(page, first);
    await expect(page.locator("[data-part='choice-number']").first()).toBeVisible();
  });
});
