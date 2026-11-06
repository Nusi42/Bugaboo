import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { inFourThemesOf, listTheOptions } from "./support/choice-pictures.ts";
import { prepareCostCenters } from "./support/cost-centers.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { issueStartingWithProductDevelopment } from "./support/time.ts";

// The window that books time in the four themes, with the choice box closed and open, for the review
// of M6. Only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareCostCenters } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M6/choice-numbers";

test("the window that books time, with the choice box closed and open", async ({
  page,
  server,
}) => {
  const issue = await issueStartingWithProductDevelopment(server);
  await signIn(page, ada);
  const window = () =>
    page.getByRole("dialog", { name: t("time.add.title", { issue: issue.key }) });
  const open = async () => {
    await openIssueWithDetails(page, issue.key);
    await page.getByRole("button", { name: t("time.add.button"), exact: true }).click();
    await expect(window()).toBeVisible();
  };
  await inFourThemesOf(page, window, { name: "add-time", milestone: MILESTONE, asIs: true }, open);
  // The open list of a box is drawn by the browser itself, outside the page; it is shown as the options one under the other.
  await inFourThemesOf(
    page,
    window,
    { name: "add-time-open", milestone: MILESTONE, asIs: true },
    async () => {
      await open();
      await listTheOptions(page);
    },
  );
});
