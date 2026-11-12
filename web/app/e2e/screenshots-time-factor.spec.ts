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
  // A time factor chosen and typed, with its sum; then one that is no time factor, with its line.
  const typed = async (text: string) => {
    await openIssue(page, factorIssues.ada);
    await change().click();
    const window = page.getByRole("dialog");
    await window.getByRole("radio", { name: t("issues.time-factor-as-factor") }).click();
    await window.getByLabel(t("issues.time-factor-factor"), { exact: true }).fill(text);
  };
  await inFourChosenThemes(page, "window-factor-sum", MILESTONE, paths.appearance, async () => {
    await typed("1.5");
    await expect(page.getByText(/= 12h/u)).toBeVisible();
  });
  await inFourChosenThemes(page, "window-factor-mistake", MILESTONE, paths.appearance, async () => {
    await typed("soon");
    await expect(page.getByText(/1[.,]25/u).last()).toBeVisible();
  });
  // Ada sets it: the row says who; then nobody's, on the issue Lea reported: as estimated.
  await setFactorAs(server, ada, factorIssues.ada, 150);
  await inFourChosenThemes(page, "row-set-by", MILESTONE, paths.appearance, async () => {
    await openIssue(page, factorIssues.ada);
    await expect(factorRow(page)).toContainText(ada.name);
  });
  await inFourChosenThemes(page, "row-as-estimated", MILESTONE, paths.appearance, async () => {
    await openIssue(page, factorIssues.lea);
    await expect(factorRow(page)).toContainText(t("issues.time-factor-as-estimated"));
  });
});
