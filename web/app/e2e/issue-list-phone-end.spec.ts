import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// The bar of the ticked issues on a phone stands in the column above the bottom navigation, so the
// end of a long list is not under it: the tick box of the last card can be pressed.

test.use({ preparation: { run: (server) => prepareManyWeb(server, 40) } });

test("ux_4_2 on a phone the list ends above the bar of the ticked issues, not under it", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the bar above the navigation");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await partIn(page, "list-row", "phone").first().locator("label").click();
  const bar = page.getByRole("region", { name: t("issues.selected-bar"), exact: true });
  await expect(bar).toBeVisible();
  // The page scrolled to its end: the box of the list, with the cards it shows, ends above the bar.
  const main = page.locator("main");
  await expect
    .poll(() =>
      main.evaluate((frame) => {
        frame.scrollTop = frame.scrollHeight;
        return frame.scrollHeight - frame.clientHeight - frame.scrollTop;
      }),
    )
    .toBeLessThanOrEqual(1);
  const [list, shown] = [await boxOf(partIn(page, "list-scroll")), await boxOf(bar)];
  expect(list.y + list.height).toBeLessThanOrEqual(shown.y + 1);
});
