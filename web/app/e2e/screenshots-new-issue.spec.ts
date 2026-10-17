import { type Page } from "@playwright/test";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { chooseTheme, themes, type Theme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { newIssueDialog, openNewIssueDialog } from "./support/issues.ts";
import { prepareLook } from "./support/list-look.ts";
import { screenshotFolder, shotWholeAs } from "./support/screenshots.ts";

// The new-issue window with its note under the buttons (UX §4.5), in the four themes, for the
// maintainer's review of M4 (WP-09). Only taken when `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-new-issue.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareLook } });

const MILESTONE = "M4/issue-page";

async function takeTheWindow(page: Page, theme: Theme, project: string) {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  await expect(newIssueDialog(page)).toBeVisible();
  await chooseTheme(page, theme);
  await shotWholeAs(page, `new-issue-${project}`, MILESTONE, theme.id);
}

for (const theme of themes) {
  test(`the new-issue window in ${theme.id}`, ({ page }, info) =>
    takeTheWindow(page, theme, info.project.name));
}
