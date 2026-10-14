import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

/**
 * Adds a relation from the issue that is open (`from`) to another issue, in words: opens the
 * form, picks what the relation means, names the other issue and waits for the sentence that
 * says it was added. The form of a relation added before stays open, so it is closed first.
 */
export async function addRelation(
  page: Page,
  from: string,
  other: string,
  meaning = "blocks",
): Promise<void> {
  const cancel = page.getByRole("button", { name: t("common.cancel"), exact: true });
  if (await cancel.isVisible()) {
    await cancel.click();
  }
  await page.getByRole("button", { name: t("related-issues.add"), exact: true }).click();
  await page.getByRole("radio", { name: meaning, exact: true }).check();
  await page.getByLabel(t("related-issues.issue-field")).fill(other);
  await page.getByRole("button", { name: t("related-issues.add-it"), exact: true }).click();
  await expect(
    page.getByText(t("related-issues.added", { key: from, name: meaning, other })),
  ).toBeVisible();
}
