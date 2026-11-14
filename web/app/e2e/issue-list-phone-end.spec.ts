import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { lastWholeCard, scrollersOf } from "./support/phone-list.ts";
import { lastRowEndAndBarTop, scrollRowsToTheEnd } from "./support/rows-end.ts";

// The bar of the ticked issues on a phone stands over the foot of the list, above the bottom
// navigation, and the list has room at its end for it: the tick box of the last card can be pressed.

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
  await scrollRowsToTheEnd(list);
  // The bar stands over the foot of the list, and the list has room at its end for it: its last
  // card ends above the bar.
  const { rowEnd, barTop } = await lastRowEndAndBarTop(list, bar);
  expect(rowEnd).toBeLessThanOrEqual(barTop + 0.5);
  // The last card that is whole in the list can be pressed.
  const last = await lastWholeCard(cards, list, barTop);
  expect(last).toBeGreaterThan(0);
  await cards.nth(last).locator("label").click();
  await expect(
    page.getByRole("status").filter({ hasText: t("issues.selected", { count: 2 }) }),
  ).toHaveCount(1);
});
