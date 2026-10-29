import { type Page } from "@playwright/test";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { rowOf } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { preparePanel } from "./support/list-panel.ts";

// The head of the list (UX §4.2; M5 exit A4, R1): every column name is shown whole, and the cells
// of a row are at least 12 px apart, so a cut name never touches the next column.

test.use({ preparation: { run: preparePanel } });

async function expectHeadWhole(page: Page, address: string): Promise<void> {
  await page.goto(address);
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await expect(partIn(page, "list-sort.label").first()).toBeVisible();
  const { cut, gaps } = await page.evaluate(() => {
    const found = { cut: [] as (string | null)[], gaps: [] as number[] };
    for (const label of document.querySelectorAll('[data-part="list-sort.label"]')) {
      if (label.scrollWidth > label.clientWidth) {
        found.cut.push(label.textContent);
      }
    }
    const head = document.querySelector('[data-part="list-row"][data-variant~="head"]');
    let before = 0;
    for (const cell of head?.children ?? []) {
      const box = cell.getBoundingClientRect();
      found.gaps.push(box.left - before);
      before = box.right;
    }
    // The first cell has no cell before it.
    found.gaps.shift();
    return found;
  });
  expect(cut).toEqual([]);
  expect(Math.min(...gaps)).toBeGreaterThanOrEqual(12);
}

for (const [name, query] of [
  // The sorted column has an arrow beside its name, which is where "Priority" was cut.
  ["alone", "?s=priority+is+not+low+sort+by+priority"],
  ["beside the panel", "?s=priority+is+not+low+sort+by+priority&open=WEB-1"],
] as const) {
  test(`ux_4_2 no column name of the head is cut, and the columns are 12 px apart, ${name} (A4, R1)`, async ({
    page,
  }) => {
    test.skip(layoutOf(page) !== "computer", "only a computer has a head");
    await signIn(page, ada);
    await expectHeadWhole(page, `${projectIssuesAddress("WEB")}${query}`);
  });
}
