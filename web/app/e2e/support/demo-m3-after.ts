import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { goodInFourThemes } from "./demo.ts";
import { made, theTwo } from "./demo-m3-state.ts";
import { signIn } from "./flows.ts";
import { inOrder } from "./sequence.ts";

// The last steps of the exit demo of M3 (`m3-exit-demo.spec.ts`): relations, what a Guest sees,
// and two people editing one description.

/** Two issues of the Website the bug is related to (the Website counts its IDs). */
const RELATED = ["WEB-1", "WEB-2"] as const;

/** Adds a relation *blocks* from the bug to another issue, in words. */
async function relate(page: Page, other: string): Promise<void> {
  // The form of the one before stays open after it was added: close it, then open it anew.
  const cancel = page.getByRole("button", { name: t("common.cancel"), exact: true });
  if (await cancel.isVisible()) {
    await cancel.click();
  }
  await page.getByRole("button", { name: t("related-issues.add"), exact: true }).click();
  await page.getByRole("radio", { name: "blocks", exact: true }).check();
  await page.getByLabel(t("related-issues.issue-field")).fill(other);
  await page.getByRole("button", { name: t("related-issues.add-it"), exact: true }).click();
  await expect(
    page.getByText(t("related-issues.added", { key: made.key, name: "blocks", other })),
  ).toBeVisible();
}

/** Step 3: the bug is related to two issues. */
export async function theBugIsRelatedToTwoIssues(page: Page): Promise<void> {
  await signIn(page, theTwo().first);
  await page.goto(issueAddress(made.key));
  await inOrder(RELATED.map((other) => () => relate(page, other)));
  const related = page.getByRole("region", { name: t("related-issues.title") });
  await inOrder(
    RELATED.map((other) => async () => {
      await expect(related.getByText(other, { exact: true })).toBeVisible();
    }),
  );
  await goodInFourThemes(page, "05-related-to-two-issues");
}
