import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { openAction } from "./support/many-changes-pages.ts";
import { serverWord } from "./support/server-words.ts";
import { prepareWorkflows, statusesOf, threeInProgress, tickAll } from "./support/workflows.ts";

// Changing many issues follows each issue's workflow (ISS-09, WF-04, M4 WP-07, UX §4.2): the bar
// offers the status changes by their words, and the preview names the issue that is skipped.

test.use({ preparation: { run: prepareWorkflows } });

test("iss_09 ticking three issues and Send to testing skips the one whose workflow does not allow it", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issues = await threeInProgress(admin);
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await tickAll(page, issues);
  const dialog = await openAction(page, t("many-changes.action-change"));
  const words = serverWord("workflow-change.send-to-testing");
  await dialog.getByLabel(t("issue-field.status"), { exact: true }).selectOption({ label: words });
  await dialog.getByRole("button", { name: t("many-changes.show-changes"), exact: true }).click();
  await expect(dialog.getByText(t("preview.summary", { changed: 2, skipped: 1 }))).toBeVisible();
  await expect(dialog.getByText(t("many-changes-skip.workflow"))).toBeVisible();
  await dialog
    .getByRole("button", { name: t("many-changes.apply", { count: 2 }), exact: true })
    .click();
  await expect(
    page.getByText(t("many-changes.result-done", { changed: 2, skipped: 0 })).first(),
  ).toBeVisible();
  expect(await statusesOf(admin, issues)).toEqual(["testing", "testing", "in-progress"]);
  await admin.dispose();
});
