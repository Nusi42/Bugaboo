import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
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

/** The heights of the rows that hold nothing but selects, the last row (it has no line under it) left out. */
function heightsOfPlainRows(matrix: Locator): Promise<readonly number[]> {
  return matrix.evaluate((node) => {
    const heights: number[] = [];
    const rows = node.querySelectorAll("tr");
    for (let at = 0; at < rows.length - 1; at += 1) {
      const row = rows[at];
      const holdsSelects = row?.querySelector("select") !== null;
      const holdsMore = row?.querySelector("button, [data-part='field.description']") !== null;
      if (row !== undefined && holdsSelects && !holdsMore) {
        heights.push(Math.round(row.getBoundingClientRect().height));
      }
    }
    return heights;
  });
}

test("ux_5 the name of a row stands level with the selects of its row", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.fieldPermissions);
  const matrix = partIn(page, "access.matrix");
  await expect(matrix).toBeVisible();
  const offsets = await offsetsOfNames(matrix);
  expect(offsets.length).toBeGreaterThan(5);
  expect(Math.max(...offsets)).toBeLessThanOrEqual(1);
});

test("ux_5 a row of selects only is as high as one control and its padding", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.fieldPermissions);
  const matrix = partIn(page, "access.matrix");
  await expect(matrix).toBeVisible();
  const heights = await heightsOfPlainRows(matrix);
  expect(heights.length).toBeGreaterThan(3);
  expect([...new Set(heights)], "the heights of the plain rows").toHaveLength(1);
});
