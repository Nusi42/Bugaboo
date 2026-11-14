import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { openIssue } from "./support/change-row.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { detailRow, prepareWeb } from "./support/issues.ts";
import { expectFocusIn, issuePageLandmarks } from "./support/landmarks.ts";
import { expectStill, movementWhile } from "./support/movement.ts";

// Adding and removing a tag moves nothing else on the issue page (UX §6.4, M8 WP-13): the row of the
// tags grows in place, what stands under it is pushed once by exactly that, and the focus stays in
// the row.

test.use({ preparation: { run: prepareWeb } });

test("ux_01_adding_and_removing_a_tag_moves_nothing_else_on_the_issue_page", async ({ page }) => {
  await signIn(page, ada);
  // A change is measured on a whole page, once every first read answered (M8-F127).
  await openIssue(page, "WEB-3");
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
  expectStill(added, still);
  await expectFocusIn(tags);
  // The History shows the change without a reload: it is read again, in place.
  await expect(
    page
      .getByRole("region", { name: t("history.title") })
      .getByText(/fresh/u)
      .first(),
  ).toBeVisible();
  const removed = await movementWhile(page, still.all, async () => {
    await tags.getByRole("button", { name: t("labels.take-off", { name: "fresh" }) }).click();
    await expect(tags.getByText("fresh", { exact: true })).toHaveCount(0);
  });
  expectStill(removed, still);
  await expectFocusIn(tags);
});
