import { t } from "@bugaboo/messages";
import type { Page } from "@playwright/test";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { newIssueDialog, openNewIssueDialog, titleLabel } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";
import {
  lengthNames,
  lengthsReversed,
  prepareTimeLayout,
  withEstimateLevel,
} from "./support/time-layout.ts";

// The lengths of time are fields like any other (TIME-03, CFG-01): the layout of the issue page
// places them, a person who may not read one never gets it whatever the layout says, and the form
// of a new issue takes an estimate when its layout places it.

test.use({ preparation: { run: prepareTimeLayout } });

/** The names of the three lengths in the order the layout of the tests places them. */
const names: readonly string[] = lengthsReversed.map((word) => lengthNames[word]);

test("time_03 the issue page shows the lengths of time where the layout places them", async ({
  page,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  const rows = partIn(page, "name-value.name");
  // The rows of Followers and of *Time spent* come from reads of their own, which end last.
  await expect(rows.filter({ hasText: t("followers.name") })).toBeVisible();
  await expect(rows.filter({ hasText: lengthNames["time-spent"] })).toBeVisible();
  const shown = await rows.allInnerTexts();
  expect(shown.filter((name) => names.includes(name))).toEqual(names);
  // They stand where the layout puts them: before *Assigned to*, *Priority* and *Reported by*,
  // which it lists after them (*Status* has no row of its own in the details).
  for (const later of [
    t("issue-field.assigned-to"),
    t("issue-field.priority"),
    t("issue-field.reported-by"),
  ]) {
    expect(shown.indexOf(lengthNames["estimated-time"])).toBeLessThan(shown.indexOf(later));
  }
  // *Time spent* is read, never typed: it has no box. *Time left* and the estimate have one.
  await expect(page.getByLabel(lengthNames["time-spent"], { exact: true })).toHaveCount(0);
  await expect(page.getByLabel(lengthNames["time-left"], { exact: true })).toBeVisible();
  await expect(page.getByLabel(lengthNames["estimated-time"], { exact: true })).toBeVisible();
});

test("time_03 a new issue is made with an estimate, and its time left starts equal to it", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  const form = newIssueDialog(page);
  await form.getByLabel(titleLabel(), { exact: true }).fill("Plan the release");
  const create = form.getByRole("button", { name: t("issues.create"), exact: true });
  await form.getByLabel(lengthNames["estimated-time"], { exact: true }).fill("1:30");
  // The box was read: the form can be sent, with no word about a mistake.
  await expect(create).toBeEnabled();
  await expect(form.getByText(t("issues.estimate-wrong"))).toHaveCount(0);
  await create.click();
  await expect(page.getByRole("heading", { name: /Plan the release/u })).toBeVisible();
  // The issue is WEB-4, after the three of the project; a small screen has the details behind a button.
  await openIssueWithDetails(page, "WEB-4");
  await expect(page.getByLabel(lengthNames["estimated-time"], { exact: true }).first()).toHaveValue(
    "1h 30m",
  );
  await expect(
    page.getByText(t("issues.time-left-worked-out", { length: "1h 30m" }), { exact: true }),
  ).toBeVisible();
});

test("time_03 a wrong length is told beside the estimate and nothing is made", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  const form = newIssueDialog(page);
  await form.getByLabel(titleLabel(), { exact: true }).fill("Wrong length");
  const estimate = form.getByLabel(lengthNames["estimated-time"], { exact: true });
  await estimate.fill("soon");
  const create = form.getByRole("button", { name: t("issues.create"), exact: true });
  await expect(estimate).toHaveAttribute("aria-invalid", "true");
  await expect(create).toBeDisabled();
  // Mended, the form can be sent again.
  await estimate.fill("2h");
  await expect(estimate).not.toHaveAttribute("aria-invalid", "true");
  await expect(create).toBeEnabled();
});

/** Bob opens the form of a new issue: it asks for the title and not for the estimate. */
async function expectFormWithoutEstimate(page: Page): Promise<void> {
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  const form = newIssueDialog(page);
  await expect(form.getByLabel(titleLabel(), { exact: true })).toBeVisible();
  await expect(form.getByLabel(lengthNames["estimated-time"], { exact: true })).toHaveCount(0);
}

test("time_03 a person who may read the estimate and not change it sees it without a box, and is not asked for it", async ({
  page,
  server,
}) => {
  await withEstimateLevel(server, "can-see", async () => {
    await signIn(page, bob);
    await openIssueWithDetails(page, "WEB-1");
    const rows = partIn(page, "name-value.name");
    await expect(rows.filter({ hasText: t("followers.name") })).toBeVisible();
    await expect(rows.filter({ hasText: lengthNames["estimated-time"] })).toBeVisible();
    await expect(page.getByLabel(lengthNames["estimated-time"], { exact: true })).toHaveCount(0);
    await expectFormWithoutEstimate(page);
  });
});

test("time_03 a person who may not read the estimate gets it nowhere, whatever the layout says", async ({
  page,
  server,
}) => {
  await withEstimateLevel(server, "hidden", async () => {
    await signIn(page, bob);
    await openIssueWithDetails(page, "WEB-1");
    const rows = partIn(page, "name-value.name");
    await expect(rows.filter({ hasText: t("followers.name") })).toBeVisible();
    await expect(rows.filter({ hasText: lengthNames["time-left"] })).toBeVisible();
    await expect(rows.filter({ hasText: lengthNames["estimated-time"] })).toHaveCount(0);
    await expectFormWithoutEstimate(page);
  });
});
