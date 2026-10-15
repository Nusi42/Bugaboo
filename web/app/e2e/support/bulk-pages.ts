import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../../src/shared/paths.ts";
import { inOrder } from "./sequence.ts";

// The screen of changing many issues (ISS-09, M3 WP-08) as a person uses it: tick issues in the
// list of the project, choose a new status, read the preview, confirm.

/** Opens the list of the project WEB and ticks the issues. */
export async function tickIssues(page: Page, keys: readonly string[]): Promise<void> {
  await page.goto(projectIssuesAddress("WEB"));
  // One after another, as a person does: the next box is ticked when the last one shows it.
  await inOrder(
    keys.map((key) => async () => {
      // A person clicks the box they see, with its key next to it, not the input behind it.
      await page
        .locator("label")
        .filter({ hasText: new RegExp(`^${key}$`, "u") })
        .click();
      await expect(page.getByRole("checkbox", { name: key, exact: true })).toBeChecked();
    }),
  );
  await expect(page.getByText(t("issues.selected", { count: keys.length }))).toBeVisible();
}

/** Chooses *Change status* to *Done* for the ticked issues and reads the preview; gives the dialog back. */
export async function previewDoneOnScreen(page: Page, count: number) {
  await page.getByRole("button", { name: t("bulk.action-change"), exact: true }).click();
  const dialog = page.getByRole("alertdialog");
  await dialog
    .getByLabel(t("issue-field.status"), { exact: true })
    .selectOption({ label: t("status.done") });
  await dialog.getByRole("button", { name: t("bulk.show-changes"), exact: true }).click();
  await expect(
    dialog.getByText(t("issues.preview-summary", { changed: count, skipped: 0 })),
  ).toBeVisible();
  return dialog;
}

/** Confirms the change and reads the result: how many changed, and that none was left alone. */
export async function applyAndExpectAllChanged(
  page: Page,
  dialog: Locator,
  changed: number,
): Promise<void> {
  await dialog.getByRole("button", { name: t("bulk.apply"), exact: true }).click();
  await expect(
    page.getByText(t("bulk.result-done", { changed, skipped: 0 })).first(),
  ).toBeVisible();
}

/** Confirms the change and reads the result: how many changed, and the issue that was left alone. */
export async function applyAndExpectSkipped(
  page: Page,
  dialog: Locator,
  changed: number,
  skipped: string,
): Promise<void> {
  await dialog.getByRole("button", { name: t("bulk.apply"), exact: true }).click();
  await expect(
    page.getByText(t("bulk.result-done", { changed, skipped: 1 })).first(),
  ).toBeVisible();
  await expect(
    page
      .getByRole("list", { name: t("bulk.skipped-list"), exact: true })
      .getByText(t("bulk.skipped-line", { issue: skipped, reason: t("bulk-skip.changed-since") }), {
        exact: true,
      }),
  ).toBeVisible();
}
