import { t } from "@bugaboo/messages";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import {
  detailRow,
  issueOf,
  prepareWeb,
  readIssue,
  setStatusAndPriority,
  versionOf,
} from "./support/issues.ts";

// Undo of a change in place (UX §4.4, R7, M3 WP-17 review): it is sent on the version the person's
// own change made, so a change that somebody else made after it is not overwritten.

test.use({ preparation: { run: prepareWeb } });

test("adr_0019 Undo does not overwrite what somebody changed after the person's own change", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, "WEB-1");
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  const row = detailRow(page, t("issue-field.priority"));
  await row.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
    label: t("priority.low"),
  });
  await expect(row.getByText(new RegExp(`^${t("save.saved")}`, "u"))).toBeVisible();
  // Somebody else changes the priority and the status after that.
  await setStatusAndPriority(admin, issue, await versionOf(admin, issue), {
    status: "open",
    priority: "critical",
  });
  await row.getByRole("button", { name: t("common.undo"), exact: true }).click();
  // The person is told who changed it first, and the other value stays.
  await expect(page.getByText(/^Not saved: .+ changed Priority to Critical/u)).toBeVisible();
  await expect(page.getByRole("button", { name: t("issues.use-mine"), exact: true })).toBeVisible();
  expect((await readIssue(admin, issue)).issue.priority).toBe("critical");
});
