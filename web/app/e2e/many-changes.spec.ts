import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { ana, signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { labelNamesOf, createLabel, tickTwo } from "./support/labels.ts";
import { openAction, previewDoneOnScreen, roomAboveBar } from "./support/many-changes-pages.ts";

// Changing many issues at once (ISS-09, M3 WP-08): tick issues, choose what to do, read what would
// change, apply, and take it back. The preview is the one long preview of the app.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("iss_09 a label is added to two ticked issues after a preview, and Undo takes it off", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await createLabel(admin, "release");
  await signIn(page, ada);
  await tickTwo(page);
  const dialog = await openAction(page, "label", 2);
  await expect(
    dialog.getByRole("button", { name: t("many-changes.apply", { count: 2 }), exact: true }),
  ).toBeDisabled();
  await dialog
    .getByLabel(t("many-changes.field-label"), { exact: true })
    .selectOption({ label: "release" });
  await dialog.getByRole("button", { name: t("many-changes.show-changes"), exact: true }).click();
  await expect(dialog.getByText(t("preview.summary", { changed: 2, skipped: 0 }))).toBeVisible();
  await expect(
    dialog.getByText(
      t("many-changes.line-changes", { field: t("issue-field.labels"), after: "release" }),
    ),
  ).toHaveCount(2);
  await dialog
    .getByRole("button", { name: t("many-changes.apply", { count: 2 }), exact: true })
    .click();
  await expect(
    page.getByText(t("many-changes.result-done", { changed: 2, skipped: 0 })).first(),
  ).toBeVisible();
  // The outcome stands under the last row of the list, in the flow: it covers no row (review of package A).
  expect(await roomAboveBar(page)).toBeGreaterThanOrEqual(0);
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
  const dialog = await previewDoneOnScreen(page, 2);
  const list = dialog.getByRole("list", { name: t("preview.label"), exact: true });
  await expect(list.getByText("WEB-1", { exact: true })).toBeVisible();
  await expect(list.getByText("WEB-2", { exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
  await dialog
    .getByRole("button", { name: t("many-changes.apply", { count: 2 }), exact: true })
    .click();
  await expect(
    page.getByText(t("many-changes.result-done", { changed: 2, skipped: 0 })).first(),
  ).toBeVisible();
  const admin = await signedInAdmin(server);
  const issue = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: "WEB-2" } } }),
  );
  expect(issue.issue.status?.key).toBe("done");
  await admin.dispose();
});

test("iss_09 an issue the person may not change is skipped with the reason and not applied", async ({
  page,
}) => {
  await signIn(page, ana);
  await tickTwo(page);
  const dialog = await openAction(page, "change", 2);
  await dialog.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
    label: t("priority.high"),
  });
  await dialog.getByRole("button", { name: t("many-changes.show-changes"), exact: true }).click();
  await expect(dialog.getByText(t("preview.summary", { changed: 0, skipped: 2 }))).toBeVisible();
  await expect(dialog.getByText(t("many-changes-skip.not-allowed"))).toHaveCount(2);
  await expect(
    dialog.getByRole("button", { name: t("many-changes.apply", { count: 0 }), exact: true }),
  ).toBeDisabled();
});

test("iss_09 the bar and its dialog are good in every theme", async ({ page }) => {
  await signIn(page, ada);
  await tickTwo(page);
  await expectGoodInEveryLook(page);
  const dialog = await openAction(page, "move", 2);
  await expect(dialog).toBeVisible();
  await expectGoodInEveryLook(page);
});
