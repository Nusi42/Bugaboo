import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
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
