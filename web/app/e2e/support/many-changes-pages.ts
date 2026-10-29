import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../../src/shared/paths.ts";
import { tickBox } from "./issues.ts";
import { inOrder } from "./sequence.ts";
import { serverWord } from "./server-words.ts";

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
        .filter({ hasText: new RegExp(`^${t("issues.select-issue", { key })}$`, "u") })
        .click();
      await expect(tickBox(page, key)).toBeChecked();
    }),
  );
  await expect(
    page.getByRole("status").filter({ hasText: t("issues.selected", { count: keys.length }) }),
  ).toBeVisible();
}

/**
 * Opens the dialog of one of the five things from the bar of the ticked issues. *Change* and
 * *Delete* carry the count (*Change 2 issues*), the others keep their name; the name given is the
 * plain one of the action (`many-changes.action-…`).
 */
export async function openAction(page: Page, name: string) {
  const counted = new Map([
    [t("many-changes.action-change"), /^Change \d+ issues?$/u],
    [t("many-changes.action-delete"), /^Delete \d+ issues?$/u],
  ]);
  await page.getByRole("button", { name: counted.get(name) ?? name, exact: true }).click();
  return page.getByRole("alertdialog");
}

/** Chooses *Change status* to *Done* for the ticked issues and reads the preview; gives the dialog back. */
export async function previewDoneOnScreen(page: Page, count: number) {
  const dialog = await openAction(page, t("many-changes.action-change"));
  await dialog
    .getByLabel(t("issue-field.status"), { exact: true })
    .selectOption({ label: serverWord("workflow-change.mark-as-done") });
  await dialog.getByRole("button", { name: t("many-changes.show-changes"), exact: true }).click();
  await expect(
    dialog.getByText(t("preview.summary", { changed: count, skipped: 0 })),
  ).toBeVisible();
  return dialog;
}

/** Confirms the change and reads the result: how many changed, and that none was left alone. */
export async function applyAndExpectAllChanged(
  page: Page,
  dialog: Locator,
  changed: number,
): Promise<void> {
  await dialog
    .getByRole("button", { name: t("many-changes.apply", { count: changed }), exact: true })
    .click();
  await expect(
    page.getByText(t("many-changes.result-done", { changed, skipped: 0 })).first(),
  ).toBeVisible();
}

/** Confirms the change and reads the result: how many changed, and the issue that was left alone. */
export async function applyAndExpectSkipped(
  page: Page,
  dialog: Locator,
  changed: number,
  skipped: string,
): Promise<void> {
  await dialog
    .getByRole("button", { name: t("many-changes.apply", { count: changed + 1 }), exact: true })
    .click();
  await expect(
    page.getByText(t("many-changes.result-done", { changed, skipped: 1 })).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("list", { name: t("many-changes.skipped-list"), exact: true }).getByText(
      t("many-changes.skipped-line", {
        issue: skipped,
        reason: t("many-changes-skip.changed-since"),
      }),
      {
        exact: true,
      },
    ),
  ).toBeVisible();
}
