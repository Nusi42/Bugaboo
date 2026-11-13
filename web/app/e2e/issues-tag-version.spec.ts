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
  rowSaved,
  versionOf,
} from "./support/issues.ts";
import { addTag } from "./support/labels.ts";

// A tag counts the version of an issue up (ADR-0017, M8 WP-13): the page takes the version of the
// answer as its own only when nobody else changed the issue in between. Otherwise it reads the issue
// again, so the person sees the other change and their next save is checked against it.

test.use({ preparation: { run: prepareWeb } });

test("adr_0017 a tag added after somebody else's change shows that change and does not overwrite it", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-3");
  await expect(
    page.getByRole("heading", { level: 1, name: /Typo on the contact page/u }),
  ).toBeVisible();
  // Somebody else changes the title while the page stands open with the version before.
  const other = await signedInAdmin(server);
  try {
    const issue = await issueOf(other, "WEB-3");
    await other.client.PATCH("/api/v1/issues/{issue}", {
      params: { path: { issue: issue.id } },
      body: { version: await versionOf(other, issue), title: "Typo fixed by somebody else" },
    });
    await addTag(detailRow(page, t("labels.tags-heading")), "fresh");
    // Only a page that sees the tag's version skip one reads the issue again: without that, the page
    // keeps the old title and takes the new version as its own (ADR-0017).
    await expect(
      page.getByRole("heading", { level: 1, name: /Typo fixed by somebody else/u }),
    ).toBeVisible();
    // The next save of the person is made on the version that holds the other change.
    await page.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
      label: t("priority.high"),
    });
    await expect(rowSaved(page, t("issue-field.priority"))).toBeVisible();
    const now = await readIssue(other, issue);
    expect(now.issue.title).toBe("Typo fixed by somebody else");
    expect(now.issue.priority).toBe("high");
  } finally {
    await other.dispose();
  }
});
