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
    await expect(page.getByText(t("issues.project-time"), { exact: false })).toBeVisible();
  });
});

/** The list of WEB with these columns or grouped by this field, as its address says. */
function listAddress(query: Readonly<Record<string, string>>): string {
  return `${projectIssuesAddress("WEB")}?${new URLSearchParams(query).toString()}`;
}

test("a list with the columns of the lengths of time", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  await goalWithTask(admin);
  await admin.dispose();
  await signIn(page, ada);
  const columns = "id,title,estimated time,time left,time spent";
  await inFourChosenThemes(page, "list-time-columns", MILESTONE, paths.appearance, async () => {
    await page.goto(listAddress({ cols: columns }));
    // A phone draws its own few columns in each card; a computer the ones the address names.
    await expect(page.getByText(t("issues.project-time"), { exact: false })).toBeVisible();
  });
});

test("a list grouped by a length of time", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  await goalWithTask(admin);
  await admin.dispose();
  await signIn(page, ada);
  await inFourChosenThemes(page, "list-time-groups", MILESTONE, paths.appearance, async () => {
    await page.goto(listAddress({ group: "estimated time" }));
    await expect(
      page.getByRole("row", {
        name: new RegExp(t("issues.length-range", { from: "4h", to: "8h" }), "u"),
      }),
    ).toBeVisible();
  });
});
