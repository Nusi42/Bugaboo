import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

/** The name of the box of the title on the form of a new issue: every required field says so. */
export function titleLabel(): string {
  return t("fields.label-required", { field: t("issue-field.title") });
}

/** The dialog that makes an issue. */
export function newIssueDialog(page: Page) {
  return page.getByRole("dialog", { name: t("issues.new-title"), exact: true });
}

/** Presses *New issue* in the bar (the sidebar on a computer, the bar of a phone); the form may still be on its way. */
export async function pressNewIssue(page: Page): Promise<void> {
  await page
    .getByRole("button", { name: t("issues.new"), exact: true })
    .first()
    .click();
  await expect(newIssueDialog(page)).toBeVisible();
}

/** The body of the dialog once its form is there: until then grey shapes stand in for it. */
function formShown(page: Page) {
  return newIssueDialog(page).locator('[data-part="window.body"][aria-busy="false"]');
}

/** Opens the dialog with the button of the bar and waits for its form. */
export async function openNewIssueDialog(page: Page): Promise<void> {
  await pressNewIssue(page);
  await expect(formShown(page)).toBeVisible();
}

/** Opens the dialog with the button in the menu of a phone. */
export async function openNewIssueDialogFromTheMenu(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: t("shell.menu") })
    .getByRole("button", { name: t("issues.new"), exact: true })
    .click();
  await expect(formShown(page)).toBeVisible();
}
