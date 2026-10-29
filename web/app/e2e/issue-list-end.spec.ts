import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { setBox } from "./support/list-keys.ts";
import { boxOf } from "./support/windows.ts";

// The end of a long list (UX §4.2; M5 exit R2): room under the last row.

/** How many issues the project WEB has when the list is long: more than a page of the list. */
const MANY = 130;

test.describe("the end of a long list", () => {
  test.use({ preparation: { run: (server) => prepareManyWeb(server, MANY) } });

  test("ux_4_2 scrolled to its end the list has room under its last row, which is whole (R2)", async ({
    page,
  }) => {
    test.skip(layoutOf(page) === "phone", "a phone scrolls the page, not the list");
    await signIn(page, ada);
    await page.goto(projectIssuesAddress("WEB"));
    const scroll = page.locator("[data-part='list-scroll']");
    await expect(scroll).toBeVisible();
    // The list loads more as it is scrolled: keep going to the end until there is no more.
    await expect
      .poll(() =>
        scroll.evaluate((box) => {
          box.scrollTop = box.scrollHeight;
          return box.scrollHeight - box.clientHeight - box.scrollTop;
        }),
      )
      .toBeLessThanOrEqual(1);
    await page.waitForTimeout(500);
    await scroll.evaluate((box) => {
      box.scrollTop = box.scrollHeight;
    });
    const rows = page.locator("[data-part='list-scroll'] [data-part='list-row']");
    const last = await boxOf(rows.last());
    const box = await boxOf(scroll);
    expect(box.y + box.height - (last.y + last.height)).toBeGreaterThanOrEqual(8);
  });

  test("ux_4_2 the bar of many ticked issues is pinned to the bottom of the window, whole, with no row under it", async ({
    page,
  }) => {
    test.skip(layoutOf(page) !== "computer", "only a computer has the bar stuck to the window");
    await signIn(page, ada);
    await page.goto(projectIssuesAddress("WEB"));
    await expect(page.locator("[data-part='list-scroll']")).toBeVisible();
    await setBox(page.getByRole("checkbox", { name: t("issues.select-shown"), exact: true }), true);
    const bar = page.getByRole("region", { name: t("issues.selected-bar"), exact: true });
    await expect(bar).toBeVisible();
    const box = await boxOf(bar);
    const height = await page.evaluate(() => window.innerHeight);
    // The bar ends at the bottom edge of the window: nothing of the list shows under it.
    expect(Math.abs(box.y + box.height - height)).toBeLessThanOrEqual(1);
  });
});
