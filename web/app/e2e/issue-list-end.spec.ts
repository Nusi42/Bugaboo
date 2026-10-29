import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
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
});
