import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

/**
 * What the first starting kind of relation reads, as the server writes it (the message
 * `relation-kind.blocks`, which only the server has: the app shows the names of the kinds as the
 * data holds them). Written once, here.
 */
export const BLOCKS = "blocks";

/**
 * Adds a relation from the issue that is open (`from`) to another issue, in words: opens the
 * form, picks what the relation means, names the other issue and waits for the sentence that
 * says it was added. The form of a relation added before stays open, so it is closed first.
 */
export async function addRelation(
  page: Page,
  from: string,
  other: string,
  meaning = BLOCKS,
): Promise<void> {
  const cancel = page.getByRole("button", { name: t("common.cancel"), exact: true });
  if (await cancel.isVisible()) {
    await cancel.click();
  }
  await page.getByRole("button", { name: t("related-issues.add"), exact: true }).click();
  await page.getByRole("radio", { name: meaning, exact: true }).check();
  await page
    .getByRole("region", { name: t("related-issues.title") })
    .getByLabel(t("common.issue-id"))
    .fill(other);
  await page.getByRole("button", { name: t("related-issues.add-it"), exact: true }).click();
  await expect(
    page.getByText(t("related-issues.added", { key: from, name: meaning, other })),
  ).toBeVisible();
}

/**
 * Makes an issue a sub-issue of the one that is open: *Add sub-issue* opens the small form, then the
 * ID goes in and the sentence that says it was done is waited for.
 */
export async function addSubIssue(page: Page, other: string): Promise<void> {
  const parts = page.getByRole("region", { name: t("sub-issues.title") });
  await parts.getByRole("button", { name: t("sub-issues.add"), exact: true }).click();
  await parts.getByLabel(t("common.issue-id")).fill(other);
  await parts.getByRole("button", { name: t("sub-issues.add-it"), exact: true }).click();
  await expect(page.getByText(t("sub-issues.added", { key: other }))).toBeVisible();
}
