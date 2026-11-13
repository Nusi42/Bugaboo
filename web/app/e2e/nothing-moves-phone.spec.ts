import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb, webIssues } from "./support/issues.ts";
import { rowOf } from "./support/list-look.ts";
import { landmarkOf, movementWhile } from "./support/movement.ts";
import { holdUntilReleased } from "./support/slow.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// Nothing moves under the cursor on a phone (UX §6.4, M8 WP-13): what the person may come back to
// stays the same element while the screen loads and reads again.

test.use({ preparation: { run: prepareWeb } });

test("ux_01_the_new_issue_button_of_a_phone_stays_the_same_while_the_list_loads", async ({
  page,
}) => {
  await signIn(page, ada);
  const held = await holdUntilReleased(page, "**/api/v1/search/fields*");
  await page.goto(projectIssuesAddress("WEB"));
  const button = page
    .locator("[data-part='search-bar']")
    .getByRole("button", { name: t("issues.new"), exact: true });
  // The focus comes back to the button when the window closes: it must be the same element after the
  // bar has loaded, not a new one in its place.
  const still = await landmarkOf(button, "New issue");
  const loading = await movementWhile(page, [still], async () => {
    held.release();
    await expect(rowOf(page, webIssues.typo)).toBeVisible();
  });
  expect({ moved: loading.moved, vanished: loading.vanished }).toEqual({ moved: [], vanished: [] });
});
