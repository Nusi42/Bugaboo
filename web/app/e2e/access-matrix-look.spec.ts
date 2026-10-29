import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { shadowsOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O2): in the tables of Field permissions and Who sees people the name of a field stands
// on the line of its selects (the middle of the words is the middle of the control), and a row is as high as
// its content needs, no more: a row without a note under its selects is one control and its padding high.

test.use({ sample: true });

/** How far the middle of the name of each row is from the middle of the first select of the row. */
function offsetsOfNames(matrix: Locator): Promise<readonly number[]> {
  return matrix.evaluate((node) => {
    const offsets: number[] = [];
    for (const row of node.querySelectorAll("tr")) {
      const name = row.querySelector("th");
      const select = row.querySelector("select");
      if (name !== null && select !== null) {
        const range = document.createRange();
        range.selectNodeContents(name);
        const words = range.getBoundingClientRect();
        const control = select.getBoundingClientRect();
        offsets.push(Math.abs(words.top + words.height / 2 - (control.top + control.height / 2)));
      }
    }
    return offsets;
  });
}

/** How many selects stand across the edge of the box that scrolls, as it is at rest: cut by it. */
function selectsCutAtTheEdge(scroll: Locator): Promise<number> {
  return scroll.evaluate((node) => {
    const edge = node.getBoundingClientRect().left + node.clientWidth;
    let cut = 0;
    for (const select of node.querySelectorAll("select")) {
      const box = select.getBoundingClientRect();
      if (box.left < edge - 0.5 && box.right > edge + 0.5) {
        cut += 1;
      }
    }
    return cut;
  });
}

/** The heights of the rows whose name is on one line (a name of several lines is a taller row), the last row left out. */
function heightsOfRows(matrix: Locator): Promise<readonly number[]> {
  return matrix.evaluate((node) => {
    const heights: number[] = [];
    const rows = node.querySelectorAll("tbody tr");
    for (let at = 0; at < rows.length - 1; at += 1) {
      const row = rows[at];
      const name = row?.querySelector("th");
      if (row !== undefined && name !== null && name !== undefined) {
        const range = document.createRange();
        range.selectNodeContents(name);
        if (range.getBoundingClientRect().height < 30) {
          heights.push(Math.round(row.getBoundingClientRect().height));
        }
      }
    }
    return heights;
  });
}

test("ux_5 every row of Field permissions and of Who sees people is one height", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone shows the levels as cards, not as a matrix");
  await signIn(page, sampleAdministrator());
  await inOrder(
    [paths.fieldPermissions, paths.peopleVisibility].map((address) => async () => {
      await page.goto(address);
      const matrix = partIn(page, "access.matrix");
      await expect(matrix).toBeVisible();
      const heights = await heightsOfRows(matrix);
      expect(heights.length).toBeGreaterThan(3);
      expect([...new Set(heights)], `${address}: the heights of the rows`).toHaveLength(1);
    }),
  );
});

test("ux_5 the name of a row stands level with the selects of its row", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone shows the levels as cards, not as a matrix");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.fieldPermissions);
  const matrix = partIn(page, "access.matrix");
  await expect(matrix).toBeVisible();
  const offsets = await offsetsOfNames(matrix);
  expect(offsets.length).toBeGreaterThan(5);
  expect(Math.max(...offsets)).toBeLessThanOrEqual(1);
});

test("ux_5 no select is cut by the edge of the table at rest", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone shows the levels as cards, not as a matrix");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.fieldPermissions);
  const scroll = partIn(page, "access.scroll");
  await expect(scroll).toBeVisible();
  expect(await selectsCutAtTheEdge(scroll), "selects cut by the right edge").toBe(0);
});

test("ux_5 the table shades its right edge while roles are hidden, and not at its end", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone shows the levels as cards, not as a matrix");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.fieldPermissions);
  const scroll = partIn(page, "access.scroll");
  await expect(scroll).toBeVisible();
  // Columns are hidden at the end: its shadow shows, the one at the start does not.
  await expect.poll(() => shadowsOf(scroll)).toEqual({ start: 0, end: 1 });
  await scroll.evaluate((node) => {
    node.scrollLeft = node.scrollWidth;
  });
  await expect.poll(() => shadowsOf(scroll)).toEqual({ start: 1, end: 0 });
});
