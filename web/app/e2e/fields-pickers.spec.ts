import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { signedInAdmin } from "./support/access.ts";
import { ada, bob } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { fieldOf, pickableGroup, preparePickers } from "./support/fields.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { createIssue, detailRow } from "./support/issues.ts";

// The pickers of own fields on the issue page (CF-01, M3 WP-05 with WP-06): a person, a group and
// an issue are chosen in a dialog and shown afterwards, and the page with its fields and a long
// text is good in all four looks.

test.use({ preparation: { run: preparePickers } });
test.describe.configure({ mode: "serial" });

/** Opens the dialog of a field by the button *Choose* that carries the name of the field. */
async function openChooser(page: Page, field: string) {
  await page
    .getByRole("button", { name: t("fields.choose-title", { name: field }), exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: t("fields.choose-title", { name: field }) });
  await expect(dialog).toBeVisible();
  return dialog;
}

test("cf_01 a person is chosen for Tester, found by name, and shown on the page", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const made = await createIssue(admin, "WEB", "Needs a tester");
  await signIn(page, ada);
  await openIssueWithDetails(page, made.key);
  const dialog = await openChooser(page, "Tester");
  await dialog.getByLabel(t("fields.pick-search"), { exact: true }).fill("Builder");
  await dialog
    .getByRole("button", { name: t("fields.pick-choose-name", { name: bob.name }), exact: true })
    .click();
  // A person is one choice: the dialog closes and the page shows the name.
  await expect(dialog).toBeHidden();
  // In its row of the Details: the history under the issue names the person too, as it is read at once.
  await expect(detailRow(page, "Tester").getByText(bob.name)).toBeVisible();
  await admin.dispose();
});

test("cf_01 a group is chosen for Team and shown on the page", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  const made = await createIssue(admin, "WEB", "Needs a team");
  await signIn(page, ada);
  await openIssueWithDetails(page, made.key);
  const dialog = await openChooser(page, "Team");
  await dialog
    .getByRole("button", {
      name: t("fields.pick-choose-name", { name: pickableGroup }),
      exact: true,
    })
    .click();
  await expect(dialog).toBeHidden();
  await expect(detailRow(page, "Team").getByText(pickableGroup)).toBeVisible();
  await admin.dispose();
});

test("cf_01 a person without Manage groups is not offered a list of groups to choose from (F62)", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const made = await createIssue(admin, "WEB", "Bob sets the fields");
  await signIn(page, bob);
  await openIssueWithDetails(page, made.key);
  // Bob may set Tester and Team, but the list of groups needs Manage groups: he can pick a
  // person, and sees no Choose for the group.
  await expect(
    page.getByRole("button", { name: t("fields.choose-title", { name: "Tester" }), exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: t("fields.choose-title", { name: "Team" }), exact: true }),
  ).toHaveCount(0);
  await admin.dispose();
});

test("cf_01 an issue is chosen for Duplicate of by its readable ID, and a wrong ID is told", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const first = await createIssue(admin, "WEB", "The first");
  const second = await createIssue(admin, "WEB", "The second");
  await signIn(page, ada);
  await openIssueWithDetails(page, second.key);
  const dialog = await openChooser(page, "Duplicate of");
  const key = dialog.getByLabel(t("common.issue-id"), { exact: true });
  await key.fill("WEB-9999");
  await dialog.getByRole("button", { name: t("fields.pick-issue-find"), exact: true }).click();
  await expect(dialog.getByRole("alert")).toBeVisible();
  await key.fill(first.key);
  await dialog.getByRole("button", { name: t("fields.pick-issue-find"), exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(
    page
      .getByRole("complementary", { name: t("issues.details") })
      .getByText(first.key, { exact: true }),
  ).toBeVisible();
  await admin.dispose();
});

test("cf_04 the issue page with own fields and a long text is good in all four looks", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const notes = await fieldOf(admin, "Notes");
  const paragraph = "Steps to see it: open the page, wait, and the window closes by itself. ";
  const longText = [
    `# Notes\n\n${paragraph.repeat(12)}`,
    `- ${"a-word-without-any-spaces-".repeat(8)}`,
    "```\n" + "a line of code that goes on and on and on ".repeat(6) + "\n```",
  ].join("\n\n");
  const made = await createIssue(admin, "WEB", "Long notes", undefined, { [notes.id]: longText });
  await signIn(page, ada);
  await openIssueWithDetails(page, made.key);
  await expect(page.getByRole("region", { name: "Notes", exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
  await admin.dispose();
});
