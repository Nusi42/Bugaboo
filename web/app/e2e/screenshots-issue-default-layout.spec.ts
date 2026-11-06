import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The issue page of an organization that changed no layout, in the four themes (TIME-03, CFG-01):
// the picture the lengths of time as fields of the layout must not change. Take it before and
// after a change to the layouts and compare the two. Only taken when `BB_SCREENSHOT_DIR` names the
// folder it goes to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

test("the issue page with the default layout", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(
    page,
    "issue-page-default",
    "M6/default-layout",
    paths.appearance,
    async () => {
      await openIssueWithDetails(page, "WEB-1");
      await expect(page.getByRole("heading", { name: /Login fails/u }).first()).toBeVisible();
    },
  );
});
