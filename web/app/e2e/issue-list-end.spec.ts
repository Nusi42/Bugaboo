import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { setBox } from "./support/list-keys.ts";
import { partIn } from "./support/parts.ts";
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

  test("ux_4_2 the bar of many ticked issues stands whole at the foot of the list in the window, with no row under it", async ({
    page,
  }) => {
    test.skip(layoutOf(page) !== "computer", "only a computer has the bar under the list");
    await signIn(page, ada);
    await page.goto(projectIssuesAddress("WEB"));
    const scroll = page.locator("[data-part='list-scroll']");
    await expect(scroll).toBeVisible();
    await setBox(page.getByRole("checkbox", { name: t("issues.select-shown"), exact: true }), true);
    const bar = page.getByRole("region", { name: t("issues.selected-bar"), exact: true });
    await expect(bar).toBeVisible();
    const [box, rows] = await Promise.all([boxOf(bar), boxOf(scroll)]);
    const height = await page.evaluate(() => window.innerHeight);
    // The list takes the rest of the window, so the bar under it is in the window whole, and the rows end above it.
    expect(box.y + box.height).toBeLessThanOrEqual(height);
    expect(rows.y + rows.height).toBeLessThanOrEqual(box.y + 0.5);
  });
});

test.describe("the end of a short list", () => {
  test.use({ preparation: { run: (server) => prepareManyWeb(server, 3) } });

  test("ux_4_2 a short list ends under its last row: the box of the rows is no higher than its rows, with no line of its own", async ({
    page,
  }) => {
    test.skip(layoutOf(page) !== "computer", "a phone draws cards");
    await signIn(page, ada);
    await page.goto(projectIssuesAddress("WEB"));
    const scroll = partIn(page, "list-scroll");
    const last = partIn(scroll, "list-row").last();
    await expect(last).toBeVisible();
    const [box, row] = await Promise.all([boxOf(scroll), boxOf(last)]);
    // Only the room at the end of the list (R2) under the last row, and no second bar under the line of that row.
    expect(box.y + box.height - (row.y + row.height)).toBeLessThanOrEqual(16);
    await expect(scroll).toHaveCSS("border-bottom-width", "0px");
  });
});
