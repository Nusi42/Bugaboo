import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb, tickBox } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// The bar of the ticked issues on a phone stands in the column above the bottom navigation, so the
// end of a long list is not under it: the tick box of the last card can be pressed.

test.use({ preparation: { run: (server) => prepareManyWeb(server, 40) } });

test("ux_4_2 on a phone the last card of a long list is not under the bar of the ticked issues", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the bar above the navigation");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await tickBox(page, "WEB-1").locator("xpath=ancestor::label").click();
  const bar = page.getByRole("region", { name: t("issues.selected-bar"), exact: true });
  await expect(bar).toBeVisible();
  // To the end of the list: it loads more as it is scrolled.
  const cards = partIn(page, "list-row", "phone");
  await expect
    .poll(async () => {
      await cards.last().scrollIntoViewIfNeeded();
      return cards.count();
    })
    .toBeGreaterThanOrEqual(43);
  await cards.last().scrollIntoViewIfNeeded();
  const [last, shown] = [await boxOf(partIn(cards.last(), "checkbox")), await boxOf(bar)];
  expect(last.y + last.height).toBeLessThanOrEqual(shown.y + 1);
});
