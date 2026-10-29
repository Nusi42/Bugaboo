import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { boxOf } from "./support/windows.ts";
import { inOrder } from "./support/sequence.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O9): on a phone the rows of the lists of fields and of labels are alike: the facts stand
// under the name in every row, not beside it in the rows where they fit.

test.use({ sample: true });

/** How many rows (by a selector) of an element have the facts beside the name and not under it. */
function rowsWithFactsBeside(area: Locator, rows: string): Promise<number> {
  return area.evaluate((node, selector) => {
    let beside = 0;
    for (const row of node.querySelectorAll(selector)) {
      const name = row.children[0];
      const facts = row.children[1];
      if (
        name !== undefined &&
        facts !== undefined &&
        facts.getBoundingClientRect().top < name.getBoundingClientRect().bottom - 0.5
      ) {
        beside += 1;
      }
    }
    return beside;
  }, rows);
}

test("ux_5 on a phone the facts of a field and of a label stand under its name in every row", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "the rows of a computer have the facts beside the name");
  await signIn(page, sampleAdministrator());
  const lists = [
    { address: paths.fields, rows: "li[data-part='item']" },
    { address: paths.labels, rows: "[data-part='list-editor.open']" },
  ] as const;
  await inOrder(
    lists.map((list) => async () => {
      await page.goto(list.address);
      await expect(page.locator(list.rows).first()).toBeVisible();
      expect(await rowsWithFactsBeside(page.locator("main"), list.rows), list.address).toBe(0);
    }),
  );
});

test("ux_5 on a phone the choices of Fields stand as a list of full-width choices", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a computer has the choices side by side");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.fields);
  const choices = page.locator("[data-part='choice-group.label']");
  await expect(choices.first()).toBeVisible();
  const [first, second, third] = [
    await boxOf(choices.nth(0)),
    await boxOf(choices.nth(1)),
    await boxOf(choices.nth(2)),
  ];
  expect(Math.round(second.width)).toBe(Math.round(first.width));
  expect(Math.round(third.width)).toBe(Math.round(first.width));
  expect(second.y).toBeGreaterThan(first.y + first.height - 1);
});
