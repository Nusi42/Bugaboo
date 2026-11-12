import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { signIn, signOut } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import {
  factorIssues,
  factorRow,
  openIssue,
  prepareTimeFactor,
  setFactorAs,
} from "./support/time-factor.ts";

// The time factor on the issue page in the four themes, on a computer and a phone (PLAN-14, UX
// §4.4): the row with the person's own estimate, the window for oneself and the window for somebody
// else. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareTimeFactor } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M8/time-factor";

test("the row and the windows of the time factor", async ({ page, server }) => {
  await setFactorAs(server, bob, factorIssues.ada, 125);
  const change = () =>
    factorRow(page).getByRole("button", { name: t("issues.time-factor-change-name") });
  await signIn(page, bob);
  await inFourChosenThemes(page, "row-own-estimate", MILESTONE, paths.appearance, async () => {
    await openIssue(page, factorIssues.ada);
    await expect(factorRow(page)).toContainText(bob.name);
  });
  await inFourChosenThemes(page, "window-own", MILESTONE, paths.appearance, async () => {
    await openIssue(page, factorIssues.ada);
    await change().click();
    await expect(page.getByRole("dialog")).toBeVisible();
  });
  // The sign-out is in the menu of the start page; the Details of a phone stand over the page.
  await page.goto(paths.home);
  await signOut(page);
  await signIn(page, ada);
  await inFourChosenThemes(page, "window-other", MILESTONE, paths.appearance, async () => {
    await openIssue(page, factorIssues.ada);
    await change().click();
    await expect(page.getByText(t("issues.time-factor-told", { name: bob.name }))).toBeVisible();
  });
});
