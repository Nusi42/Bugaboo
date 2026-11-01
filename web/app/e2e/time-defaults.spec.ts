import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { prepareFieldsIssue } from "./support/fields.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { createIssue, newIssueDialog, openNewIssueDialog, savedNote } from "./support/issues.ts";
import { serverWord } from "./support/server-words.ts";

// The default of an issue for the time booked on it (ADR-0035, decision 1, UX §4.4 Accounting):
// the fields of time entries, Activity and Cost center, are no fields of the issue. The new-issue
// form does not ask for them, and the details of the issue show them in a group of their own.

test.use({ preparation: { run: prepareFieldsIssue } });

const activity = serverWord("starter-field.activity");
const costCenter = serverWord("starter-field.cost-center");

test("time_13 the new-issue form does not ask for Activity or Cost center", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  const form = newIssueDialog(page);
  await expect(form.getByText(t("issues.create"), { exact: true })).toBeVisible();
  await expect(form.getByText(activity, { exact: true })).toHaveCount(0);
  await expect(form.getByText(costCenter, { exact: true })).toHaveCount(0);
  await expect(form.getByText(t("fields.time-defaults"), { exact: true })).toHaveCount(0);
  await expectGoodInEveryLook(page);
});

test("time_13 the details of an issue show its defaults for booked time under their own label", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const made = await createIssue(admin, "WEB", "Paper runs out");
  await admin.dispose();
  await signIn(page, ada);
  await openIssueWithDetails(page, made.key);
  const group = page.getByRole("region", { name: t("fields.time-defaults"), exact: true });
  await expect(group.getByRole("heading", { name: t("fields.time-defaults") })).toBeVisible();
  const chosen = group.getByLabel(activity, { exact: true });
  await expect(chosen).toBeVisible();
  await expect(group.getByRole("term").filter({ hasText: costCenter })).toBeVisible();
  // It is changed there, like a field of the issue.
  await chosen.selectOption({ label: serverWord("starter-field.activity-testing") });
  await expect(savedNote(page)).toBeVisible();
  // Kept: the page opened again shows it.
  await openIssueWithDetails(page, made.key);
  await expect(
    page
      .getByRole("region", { name: t("fields.time-defaults"), exact: true })
      .getByLabel(activity, { exact: true }),
  ).toHaveValue(/.+/u);
  await expectGoodInEveryLook(page);
});
