import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { changeButton, openReleases, prepareReleases, releases } from "./support/releases.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The releases of a project and the two fields of an issue in the four themes (UX §4.16, ISS-08): the page
// with its tables, the window of a release, the issue page with its picker of releases. Only taken when
// `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareReleases } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M10/releases";

test("the page Releases, the window of a release and the picker of an issue", async ({ page }) => {
  await signIn(page, ada);
  await openReleases(page);
  await expect(changeButton(page, releases.named)).toBeVisible();
  await inFourThemes(page, "releases", MILESTONE);
  await changeButton(page, releases.named).click();
  await inFourThemesAsWindow(page, "release-window", MILESTONE);
  await page.keyboard.press("Escape");
  await openIssueWithDetails(page, "WEB-1");
  await expect(page.getByText(releases.named, { exact: true })).toBeVisible();
  await inFourThemes(page, "issue-page", MILESTONE);
  await page
    .getByRole("button", {
      name: t("releases.choose-one-for", { field: t("issue-field.found-in-release") }),
    })
    .click();
  await inFourThemesAsWindow(page, "picker", MILESTONE);
});
