import { t } from "@bugaboo/messages";

import { layoutsAddress, paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourChosenThemes } from "./support/four-themes.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { newIssueDialog, openNewIssueDialog, titleLabel } from "./support/issues.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { prepareTimeLayout } from "./support/time-layout.ts";

// The lengths of time as fields of the layouts (TIME-03, CFG-01), in the four themes, for the
// review of the screens: the layout editor with the three fields, the issue page that follows the
// layout, and the form of a new issue with an estimate. They are only taken when
// `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareTimeLayout } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M6/time-layout";

test("the layout editor of the issue page with the lengths of time", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "layout-editor", MILESTONE, paths.appearance, async () => {
    await page.goto(layoutsAddress("WEB", { form: "issue-page" }));
    await expect(
      page.getByRole("heading", { name: t("layouts.section", { number: "1" }) }),
    ).toBeVisible();
    await expect(page.getByText(t("issue-field.estimated-time"), { exact: true })).toBeVisible();
  });
});

test("the issue page with the lengths of time where the layout places them", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "issue-page", MILESTONE, paths.appearance, async () => {
    await openIssueWithDetails(page, "WEB-1");
    await expect(page.getByText(t("issue-field.time-spent"), { exact: true })).toBeVisible();
  });
});

test("the form of a new issue with an estimate", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "new-issue-estimate", MILESTONE, paths.appearance, async () => {
    await page.goto(projectIssuesAddress("WEB"));
    await openNewIssueDialog(page);
    const form = newIssueDialog(page);
    await form.getByLabel(titleLabel(), { exact: true }).fill("Plan the release");
    const estimate = form.getByLabel(t("issue-field.estimated-time"), { exact: true });
    await estimate.fill("1:30");
    // The hint under the box is the end of the form: scrolled to, it shows the box whole above it.
    const hint = form.getByText(t("time.entry.length-example-short"), { exact: true });
    await hint.scrollIntoViewIfNeeded();
    await expect(hint).toBeVisible();
  });
});

test("add time with the hint under the length", async ({ page }) => {
  await signIn(page, ada);
  await inFourChosenThemes(page, "add-time-hint", MILESTONE, paths.appearance, async () => {
    await openIssueWithDetails(page, "WEB-1");
    await page.getByRole("button", { name: t("time.add.button"), exact: true }).click();
    const dialog = page.getByRole("dialog", { name: t("time.add.title", { issue: "WEB-1" }) });
    await expect(
      dialog.getByText(t("time.entry.length-example-short"), { exact: true }),
    ).toBeVisible();
  });
});
