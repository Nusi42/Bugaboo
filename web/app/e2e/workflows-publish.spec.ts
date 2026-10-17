import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import { issueOf, setStatusAndPriority, versionOf } from "./support/issues.ts";
import { statusOf } from "./support/many-changes.ts";
import { prepareWorkflows } from "./support/workflows.ts";
import {
  blockOf,
  openSimpleToChange,
  removeStatus,
  renameWorkflowTo,
} from "./support/workflows-pages.ts";

// Publishing a workflow (WF-02, ADR-0031 amendments 11 and 12, M4-F42, M4-F43): the editor asks
// where the issues of a removed status go, names the ones the person sees, and keeps the changes
// when somebody else published first.

test.use({ preparation: { run: prepareWorkflows } });
test.describe.configure({ mode: "serial" });

test("wf_02 Publish asks where the issues of a removed status go and moves them", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, "WEB-1");
  await setStatusAndPriority(admin, issue, await versionOf(admin, issue), {
    status: "in-progress",
    priority: "normal",
  });
  await signIn(page, ada);
  await openSimpleToChange(page);
  await removeStatus(page, "In progress");
  await expect(blockOf(page, "In progress")).toHaveCount(0);
  await page.getByRole("button", { name: t("workflows.publish"), exact: true }).click();
  const question = page.getByRole("alertdialog", { name: t("workflows.publish-title") });
  await expect(
    question.getByText(t("workflows.publish-where", { count: 1, status: "In progress" })),
  ).toBeVisible();
  await expect(question.getByText(/^WEB-1 · /u)).toBeVisible();
  await expectGoodInEveryLook(page);
  await question
    .getByRole("button", { name: t("workflows.publish-go", { count: 1 }), exact: true })
    .click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await expect.poll(() => statusOf(admin, issue)).toBe("open");
  await admin.dispose();
});

test("wf_02 a draft is kept when somebody else published first, and put back on request", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ada);
  await openSimpleToChange(page);
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await openSimpleToChange(other);
  await renameWorkflowTo(other, "Simple, theirs");
  await other.getByRole("button", { name: t("workflows.publish"), exact: true }).click();
  await expect(other.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await renameWorkflowTo(page, "Simple, mine");
  await page.getByRole("button", { name: t("workflows.publish"), exact: true }).click();
  await expect(page.getByText(t("workflows.draft-changed"))).toBeVisible();
  await expect(page.getByLabel(t("workflows.name"), { exact: true })).toHaveValue("Simple, theirs");
  await page.getByRole("button", { name: t("workflows.draft-restore"), exact: true }).click();
  await expect(page.getByLabel(t("workflows.name"), { exact: true })).toHaveValue("Simple, mine");
});
