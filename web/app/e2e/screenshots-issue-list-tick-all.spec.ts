import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { grid } from "./support/list-look.ts";
import { listWindowInFourThemes, tickShown } from "./support/list-pictures.ts";
import { prepareLong } from "./support/list-pictures-data.ts";

// The pictures of the offer to tick every result and of the sentence that asks to narrow the
// search (M5 WP-08). Taken only when `BB_SCREENSHOT_DIR` is set; the builder looks at each.

test.use({ preparation: { run: prepareLong } });
test.describe.configure({ mode: "serial" });

test("cfg_02 pictures: tick all results, and narrow the search", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "the tick boxes of a phone are in the cards");
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?group=-`);
  await expect(grid(page).getByRole("link").first()).toBeVisible();
  // The list may still be reading a page while the head is ticked: tick again until every row shown is.
  const offer = page.getByRole("button", { name: t("issues.tick-all", { count: 133 }) });
  await expect(async () => {
    await tickShown(page);
    await expect(offer).toBeVisible({ timeout: 2000 });
  }).toPass();
  await listWindowInFourThemes(page, "tick-all");
  // More issues than one change may name: the bar says to narrow the search.
  await page.route("**/api/v1/search/issues/count*", (route) =>
    route.fulfill({ json: { issues: 6000 } }),
  );
  await page.reload();
  const narrow = page.getByText(t("issues.tick-narrow", { limit: 5000 }));
  await expect(async () => {
    await tickShown(page);
    await expect(narrow).toBeVisible({ timeout: 2000 });
  }).toPass();
  await listWindowInFourThemes(page, "tick-narrow");
});
