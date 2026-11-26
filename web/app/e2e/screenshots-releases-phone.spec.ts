import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { changeButton, openReleases, prepareReleases, releases } from "./support/releases.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The releases of a project on a phone in the four themes (UX §4.16, ISS-08): the page as a list and the
// window of a release. Only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareReleases } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M10/releases";

test("the page Releases and the window of a release on a phone", async ({ page }) => {
  await signIn(page, ada);
  await openReleases(page);
  await expect(changeButton(page, releases.named)).toBeVisible();
  await inFourThemes(page, "releases", MILESTONE);
  await changeButton(page, releases.named).click();
  await inFourThemesAsWindow(page, "release-window", MILESTONE);
});
