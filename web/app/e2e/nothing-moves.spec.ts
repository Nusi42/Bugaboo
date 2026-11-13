import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { detailRow, prepareWeb } from "./support/issues.ts";
import { expectFocusIn, issuePageLandmarks } from "./support/landmarks.ts";
import { expectStill, movementWhile } from "./support/movement.ts";

// Nothing moves under the cursor (UX §6, M8 WP-13): a change a person makes changes only what they
// changed; nothing else on the screen moves, flashes, empties or is drawn afresh, and the focus stays
// where the person works.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("ux_01_adding_and_removing_a_tag_moves_nothing_else_on_the_issue_page", async ({ page }) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-3");
  const tags = detailRow(page, t("labels.tags-heading"));
  await tags.scrollIntoViewIfNeeded();
  // What the person opens and types before the change is theirs; the change starts with *Add tag*.
  await tags.getByRole("button", { name: t("labels.add-tag-button"), exact: true }).click();
  await tags.getByLabel(t("labels.add-tag"), { exact: true }).fill("fresh");
  const still = await issuePageLandmarks(page);
  const added = await movementWhile(page, still.all, async () => {
    await tags.getByRole("button", { name: t("labels.add-tag-button"), exact: true }).click();
    await expect(tags.getByText(new RegExp(`^${t("save.saved")}`, "u"))).toBeVisible();
  });
  expectStill(added, still.below);
  await expectFocusIn(tags);
  const removed = await movementWhile(page, still.all, async () => {
    await tags.getByRole("button", { name: t("labels.take-off", { name: "fresh" }) }).click();
    await expect(tags.getByText("fresh", { exact: true })).toHaveCount(0);
  });
  expectStill(removed, still.below);
  await expectFocusIn(tags);
});
