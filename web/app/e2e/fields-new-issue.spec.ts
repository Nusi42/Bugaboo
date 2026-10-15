import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada, bob } from "./support/accounts.ts";
import { preparePickers } from "./support/fields.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { newIssueDialog, openNewIssueDialog } from "./support/issues.ts";
import { addToTheFormOfWeb } from "./support/layouts.ts";

// An own field of people on the form of a new issue (CF-01, M3-F104): the form shows who was
// picked by name, and not how many, until the issue is made.

test.use({ preparation: { run: preparePickers } });

test("cf_01 on the form of a new issue a person who was picked is shown by name", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await addToTheFormOfWeb(admin, ["Tester"]);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  const form = newIssueDialog(page);
  await form.getByLabel(t("issues.title-required"), { exact: true }).fill("Needs a tester");
  await form
    .getByRole("button", { name: t("fields.choose-title", { name: "Tester" }), exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: t("fields.choose-title", { name: "Tester" }) });
  await dialog.getByLabel(t("fields.pick-search"), { exact: true }).fill("Builder");
  await dialog
    .getByRole("button", { name: t("fields.pick-choose-name", { name: bob.name }), exact: true })
    .click();
  await expect(dialog).toBeHidden();
  // The name, not how many were chosen.
  await expect(form.getByText(bob.name)).toBeVisible();
  await expect(form.getByText(t("fields.chosen-count", { count: 1 }))).toHaveCount(0);
  await form.getByRole("button", { name: t("issues.create"), exact: true }).click();
  await expect(page.getByRole("heading", { name: /Needs a tester/u })).toBeVisible();
  await expect(page.getByText(bob.name).first()).toBeVisible();
});
