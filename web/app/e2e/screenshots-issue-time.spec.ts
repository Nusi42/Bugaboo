import { t } from "@bugaboo/messages";

import { paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { goalWithTask } from "./support/issue-time.ts";
import { prepareWeb } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The lengths of time of an issue in its Details, and the totals of a project above its list
// (TIME-03), in the four themes, for the review of the screens. They are only taken when
// `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M6/issue-time";

test("the Details of an issue with its estimate, time left, time spent and totals", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const goal = await goalWithTask(admin);
  await admin.dispose();
  await signIn(page, ada);
  await inFourChosenThemes(page, "issue-time", MILESTONE, paths.appearance, async () => {
    await openIssueWithDetails(page, goal.key);
    await expect(page.getByText(t("issues.with-sub-issues"), { exact: true })).toBeVisible();
  });
});

test("the list of a project with the time in the project", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  await goalWithTask(admin);
  await admin.dispose();
  await signIn(page, ada);
  await inFourChosenThemes(page, "project-time", MILESTONE, paths.appearance, async () => {
    await page.goto(projectIssuesAddress("WEB"));
    await expect(
      page.getByText(t("issues.project-time", { totals: "" }), { exact: false }),
    ).toBeVisible();
  });
});
