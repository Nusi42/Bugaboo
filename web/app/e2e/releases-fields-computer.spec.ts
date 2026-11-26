import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { newIssueDialog, openNewIssueDialog, titleLabel } from "./support/issues.ts";
import { addToTheFormOfWeb } from "./support/layouts.ts";
import { prepareReleases, releases } from "./support/releases.ts";

// *Found in release* and *Planned for release* on the issue page and on the form of a new issue (ISS-08,
// ADR-0049): the picker of releases, one release for the second and several for the first.

test.use({ preparation: { run: prepareReleases } });

const planned = t("issue-field.planned-for-release");
const found = t("issue-field.found-in-release");

/** The button that opens the picker of a field of releases, named by the field. */
function changeOf(scope: ReturnType<typeof newIssueDialog>, field: string) {
  return scope.getByRole("button", { name: t("releases.choose-one-for", { field }), exact: true });
}

test("iss_08_found_in_and_planned_for_are_set_on_the_issue_page_and_the_new_issue_form", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await addToTheFormOfWeb(admin, ["planned-for-release", "found-in-release"]);
  await admin.dispose();
  await signIn(page, ada);
  // The issue page: one release for the plan, several for where it was found.
  await openIssueWithDetails(page, "WEB-2");
  await page
    .getByRole("button", { name: t("releases.choose-one-for", { field: planned }) })
    .click();
  const one = page.getByRole("dialog", { name: planned });
  await one
    .getByRole("button", { name: t("releases.choose-name", { name: releases.open }) })
    .click();
  await expect(one).toBeHidden();
  await expect(page.getByText(releases.open, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: t("releases.choose-one-for", { field: found }) }).click();
  const several = page.getByRole("dialog", { name: found });
  await several
    .getByRole("button", { name: t("releases.choose-name", { name: releases.open }) })
    .click();
  await several
    .getByRole("button", { name: t("releases.choose-name", { name: releases.named }) })
    .click();
  await several.getByRole("button", { name: t("common.done") }).click();
  await expect(page.getByText(releases.named, { exact: true })).toBeVisible();
  // The form of a new issue has the same two fields and the same picker.
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  const form = newIssueDialog(page);
  await form.getByLabel(titleLabel(), { exact: true }).fill("Planned for a release");
  await changeOf(form, planned).click();
  await page
    .getByRole("dialog", { name: planned })
    .getByRole("button", { name: t("releases.choose-name", { name: releases.named }) })
    .click();
  await expect(form.getByText(releases.named, { exact: true })).toBeVisible();
  await form.getByRole("button", { name: t("issues.create"), exact: true }).click();
  await expect(page.getByRole("heading", { name: /Planned for a release/u })).toBeVisible();
  await expect(page.getByText(releases.named, { exact: true })).toBeVisible();
});
