import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress, projectIssuesAddress } from "../../src/shared/paths.ts";
import { signedInAdmin } from "./access.ts";
import { inFourThemes } from "./four-themes.ts";
import { issueOf, statusButton } from "./issues.ts";
import { openAction } from "./many-changes-pages.ts";
import { serverWord } from "./server-words.ts";
import { type Instance } from "./server.ts";
import { createBug, startWorkingOn, tickAll } from "./workflows.ts";

/** Three ticked issues with *Send to testing* in the preview, and the list of a Bug. */
export async function shootManyIssues(page: Page, server: Instance): Promise<void> {
  const admin = await signedInAdmin(server);
  const bug = await createBug(admin, "A bug that has just been reported");
  // The Task is still Open, and no workflow of Task has a way from Open to Testing.
  const bugs = [bug, await createBug(admin, "A second bug")];
  await startWorkingOn(admin, bugs);
  const issues = [...bugs, await issueOf(admin, "WEB-3")];
  await admin.dispose();
  await page.goto(projectIssuesAddress("WEB"));
  await tickAll(page, issues);
  const dialog = await openAction(page, t("many-changes.action-change"));
  await dialog
    .getByLabel(t("issue-field.status"), { exact: true })
    .selectOption({ label: serverWord("workflow-change.send-to-testing") });
  await dialog.getByRole("button", { name: t("many-changes.show-changes"), exact: true }).click();
  await expect(dialog.getByText(t("many-changes-skip.workflow"))).toBeVisible();
  await inFourThemes(page, "30-many-issues");
  await page.goto(issueAddress(bug.key));
  await statusButton(page).click();
  await inFourThemes(page, "31-bug-status-list");
}
