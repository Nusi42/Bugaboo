import { t } from "@bugaboo/messages";

import { paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { prepareFieldsIssue } from "./support/fields.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes, inFourThemes } from "./support/four-themes.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { createIssue, openNewIssueDialog } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The details of an issue with its defaults for booked time, and the new-issue form without them
// (ADR-0035, decision 1), in the four themes, for the maintainer's review of M6. They are only
// taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareFieldsIssue } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M6/time-defaults";

test("the details of an issue with its defaults for booked time", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const made = await createIssue(admin, "WEB", "Printer jams on the second page");
  await admin.dispose();
  await signIn(page, ada);
  // The issue page reads the person's appearance again, so each theme is chosen as a person does.
  await inFourChosenThemes(page, "issue-details", MILESTONE, paths.appearance, async () => {
    await openIssueWithDetails(page, made.key);
    await expect(
      page.getByRole("region", { name: t("fields.time-defaults"), exact: true }),
    ).toBeVisible();
  });
});

test("the new-issue form, which does not ask for them", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  await inFourThemes(page, "new-issue", MILESTONE);
});
