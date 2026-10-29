import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { lastWholeCard, scrollersOf, toTheEnd } from "./support/phone-list.ts";
import { boxOf } from "./support/windows.ts";

// The bar of the ticked issues on a phone stands in the column above the bottom navigation, so the
// end of a long list is not under it: the tick box of the last card can be pressed.

test.use({ preparation: { run: (server) => prepareManyWeb(server, 40) } });

test("ux_4_2 on a phone the list is the one scroller, and its last card can be pressed above the ticked bar", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the bar above the navigation");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const cards = partIn(page, "list-row", "phone");
  await cards.first().locator("label").click();
  const bar = page.getByRole("region", { name: t("issues.selected-bar"), exact: true });
  await expect(bar).toBeVisible();
  // One scroller: the page does not scroll, and nothing but the list does.
  expect(await scrollersOf(page)).toEqual(["list-scroll"]);
  // To the end of the list: it loads and draws more as it goes.
  const list = partIn(page, "list-scroll");
  await expect.poll(() => toTheEnd(list)).toBeLessThanOrEqual(1);
  const [box, shown] = [await boxOf(list), await boxOf(bar)];
  // The list ends above the bar, not under it.
  expect(box.y + box.height).toBeLessThanOrEqual(shown.y + 1);
  // The last card that is whole in the list can be pressed.
  const last = await lastWholeCard(cards, list);
  expect(last).toBeGreaterThan(0);
  await cards.nth(last).locator("label").click();
  await expect(
    page.getByRole("status").filter({ hasText: t("issues.selected", { count: 2 }) }),
  ).toHaveCount(1);
});
