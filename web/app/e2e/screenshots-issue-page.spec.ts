import { t } from "@bugaboo/messages";

import { type Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { chooseTheme, themes, type Theme } from "./support/checks.ts";
import { chooseFiles, screenshot } from "./support/files.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { prepareLook } from "./support/list-look.ts";
import { screenshotFolder, shotWholeAs } from "./support/screenshots.ts";

// The issue page with its Details, Followers and Files, in the four themes, for the maintainer's review
// of M4 (WP-09). Only taken when `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-issue-page.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

const MILESTONE = "M4/issue-page";
const ISSUE = "WEB-1";

/** The state the pictures start from: a file on the issue, and the person following it. */
test("the issue gets a file and a follower", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress(ISSUE));
  await chooseFiles(
    page,
    page
      .getByRole("region", { name: t("files.title"), exact: true })
      .getByRole("button", { name: t("files.upload"), exact: true }),
    [screenshot("screenshot.png", 480, 300)],
  );
  await expect(
    page.getByRole("img", { name: t("files.preview-of", { name: "screenshot.png" }) }),
  ).toBeVisible({ timeout: 30_000 });
});

async function takeThePage(page: Page, theme: Theme, project: string) {
  await signIn(page, ada);
  await openIssueWithDetails(page, ISSUE);
  await expect(page.getByText("screenshot.png").first()).toBeVisible();
  await chooseTheme(page, theme);
  await shotWholeAs(page, `issue-${project}`, MILESTONE, theme.id);
}

for (const theme of themes) {
  test(`the issue page in ${theme.id}`, ({ page }, info) =>
    takeThePage(page, theme, info.project.name));
}
