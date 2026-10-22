import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { chooseTheme, themes, type Theme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { detailRow, rowSaved } from "./support/issues.ts";
import { addTag } from "./support/labels.ts";
import { prepareLook } from "./support/list-look.ts";
import { screenshotFolder, shotWholeAs } from "./support/screenshots.ts";

// A row of the Details that says Saved with Undo (R7), in the four themes, for the maintainer's review
// of M4 (WP-09). Only taken when `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-issue-row.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

const MILESTONE = "M4/issue-page";
const ISSUE = "WEB-1";

/** A tag was just added: its row says Saved, with Undo; the tag is taken off again after. */
async function takeTheSavedRow(page: Page, theme: Theme, project: string) {
  await signIn(page, ada);
  await openIssueWithDetails(page, ISSUE);
  const tags = detailRow(page, t("labels.tags-heading"));
  await addTag(tags, "urgent");
  await expect(rowSaved(page, t("labels.tags-heading"))).toBeVisible();
  await chooseTheme(page, theme);
  await shotWholeAs(page, `issue-tag-saved-${project}`, MILESTONE, theme.id);
  await tags.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect(tags.getByText("urgent", { exact: true })).toHaveCount(0);
}

for (const theme of themes) {
  test.describe(`the saved row in ${theme.id}`, () => {
    // Each theme starts on fresh data: the history of WEB-1 shows this tag and no other.
    test.use({ freshData: theme.id });
    test(`a tag was added in ${theme.id}`, ({ page }, info) =>
      takeTheSavedRow(page, theme, info.project.name));
  });
}
