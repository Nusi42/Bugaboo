import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { goodInFourThemes } from "./demo.ts";
import { made, theTwo } from "./demo-m3-state.ts";
import { signIn } from "./flows.ts";
import { addRelation } from "./relate.ts";
import { inOrder } from "./sequence.ts";

// The last steps of the exit demo of M3 (`m3-exit-demo.spec.ts`): relations, what a Guest sees,
// and two people editing one description.

/** Two issues of the Website the bug is related to (the Website counts its IDs). */
const RELATED = ["WEB-1", "WEB-2"] as const;

/** Step 3: the bug is related to two issues. */
export async function theBugIsRelatedToTwoIssues(page: Page): Promise<void> {
  await signIn(page, theTwo().first);
  await page.goto(issueAddress(made.key));
  await inOrder(RELATED.map((other) => () => addRelation(page, made.key, other)));
  const related = page.getByRole("region", { name: t("related-issues.title") });
  await inOrder(
    RELATED.map((other) => async () => {
      await expect(related.getByText(other, { exact: true })).toBeVisible();
    }),
  );
  await goodInFourThemes(page, "05-related-to-two-issues");
}
