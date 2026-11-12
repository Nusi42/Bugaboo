import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { showDetails } from "./support/labels.ts";
import {
  newIssueDialog,
  openNewIssueDialog,
  prepareWeb,
  rowSaved,
  titleLabel,
} from "./support/issues.ts";

// The two dates of an issue (PLAN-01, ADR-0043, M3-F101): the form of a new issue asks for them, and
// *Due date* no longer waits under the fields that are not on the form.

test.use({ preparation: { run: prepareWeb } });

test("plan_01 the due date can be set on the new issue form", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  const form = newIssueDialog(page);
  await form.getByLabel(titleLabel(), { exact: true }).fill("Ship the plan");
  await form.getByLabel(t("issue-field.start-date"), { exact: true }).fill("2026-11-03");
  await form.getByLabel(t("issue-field.due-date"), { exact: true }).fill("2026-11-07");
  await form.getByRole("button", { name: t("issues.create"), exact: true }).click();
  await expect(page.getByRole("heading", { name: /Ship the plan/u })).toBeVisible();
  // The issue page shows both, and each is a box that changes in place.
  await showDetails(page);
  await expect(page.getByLabel(t("issue-field.start-date"), { exact: true })).toHaveValue(
    /2026|03/u,
  );
  const due = page.getByLabel(t("issue-field.due-date"), { exact: true });
  await expect(due).toHaveValue(/07|2026/u);
  // A typed day is a row of the Details with no Save button: leaving the box keeps it.
  await due.fill("2026-11-14");
  await due.blur();
  await expect(rowSaved(page, t("issue-field.due-date"))).toBeVisible();
});

test("plan_01 a start after the due date is refused and says so", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  const form = newIssueDialog(page);
  await form.getByLabel(titleLabel(), { exact: true }).fill("Backwards");
  await form.getByLabel(t("issue-field.start-date"), { exact: true }).fill("2026-11-10");
  await form.getByLabel(t("issue-field.due-date"), { exact: true }).fill("2026-11-03");
  await form.getByRole("button", { name: t("issues.create"), exact: true }).click();
  await expect(form.getByText(t("error.issue.start-after-due"))).toBeVisible();
});
