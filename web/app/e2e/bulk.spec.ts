import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { ana, signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { labelNamesOf, makeLabel, tickTwo } from "./support/labels.ts";

// Changing many issues at once (ISS-09, M3 WP-08): tick issues, choose what to do, read what would
// change, apply, and take it back. The preview is the one long preview of the app.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** Opens the dialog of one of the five things. */
async function openAction(page: Page, name: string) {
  await page.getByRole("button", { name, exact: true }).click();
  return page.getByRole("alertdialog");
}

test("iss_09 a label is added to two ticked issues after a preview, and Undo takes it off", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await makeLabel(admin, "release");
  await signIn(page, ada);
  await tickTwo(page);
  const dialog = await openAction(page, t("bulk.action-label"));
  await expect(dialog.getByRole("button", { name: t("bulk.apply"), exact: true })).toBeDisabled();
  await dialog
    .getByLabel(t("bulk.field-label"), { exact: true })
    .selectOption({ label: "release" });
  await dialog.getByRole("button", { name: t("bulk.show-changes"), exact: true }).click();
  await expect(
    dialog.getByText(t("issues.preview-summary", { changed: 2, skipped: 0 })),
  ).toBeVisible();
  await expect(
    dialog.getByText(t("bulk.line-changes", { field: t("issue-field.labels"), after: "release" })),
  ).toHaveCount(2);
  await dialog.getByRole("button", { name: t("bulk.apply"), exact: true }).click();
  await expect(
    page.getByText(t("bulk.result-done", { changed: 2, skipped: 0 })).first(),
  ).toBeVisible();
  expect(await labelNamesOf(admin, "WEB-1")).toEqual(["release"]);
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect.poll(() => labelNamesOf(admin, "WEB-1")).toEqual([]);
  await admin.dispose();
});

test("iss_09 the status of two ticked issues is changed and the preview names each issue", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await tickTwo(page);
  const dialog = await openAction(page, t("bulk.action-change"));
  await dialog.getByLabel(t("issue-field.status"), { exact: true }).selectOption({
    label: t("status.done"),
  });
  await dialog.getByRole("button", { name: t("bulk.show-changes"), exact: true }).click();
  const list = dialog.getByRole("list", { name: t("issues.preview-label"), exact: true });
  await expect(list.getByText("WEB-1", { exact: true })).toBeVisible();
  await expect(list.getByText("WEB-2", { exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
  await dialog.getByRole("button", { name: t("bulk.apply"), exact: true }).click();
  await expect(
    page.getByText(t("bulk.result-done", { changed: 2, skipped: 0 })).first(),
  ).toBeVisible();
  const admin = await signedInAdmin(server);
  const issue = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: "WEB-2" } } }),
  );
  expect(issue.issue.status).toBe("done");
  await admin.dispose();
});

test("iss_09 an issue the person may not change is skipped with the reason and not applied", async ({
  page,
}) => {
  await signIn(page, ana);
  await tickTwo(page);
  const dialog = await openAction(page, t("bulk.action-change"));
  await dialog.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
    label: t("priority.high"),
  });
  await dialog.getByRole("button", { name: t("bulk.show-changes"), exact: true }).click();
  await expect(
    dialog.getByText(t("issues.preview-summary", { changed: 0, skipped: 2 })),
  ).toBeVisible();
  await expect(dialog.getByText(t("bulk-skip.not-allowed"))).toHaveCount(2);
  await expect(dialog.getByRole("button", { name: t("bulk.apply"), exact: true })).toBeDisabled();
});

test("iss_09 the bar and its dialog are good in every theme", async ({ page }) => {
  await signIn(page, ada);
  await tickTwo(page);
  await expectGoodInEveryLook(page);
  const dialog = await openAction(page, t("bulk.action-move"));
  await expect(dialog).toBeVisible();
  await expectGoodInEveryLook(page);
});
