import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O16): a row that opens something has one chevron, at the end of the row, in the middle of
// its height, never on a line of its own: in every list and on both a computer and a phone.

test.use({ sample: true });

/** The chevrons of a row, how far the first stands from the end of the row, and whether it stands on a line of its own. */
function chevronOf(row: Locator): Promise<{ count: number; fromEnd: number; alone: boolean }> {
  return row.evaluate((node) => {
    const chevrons = node.querySelectorAll("svg.lucide-chevron-right");
    const box = node.getBoundingClientRect();
    const first = chevrons.item(0)?.getBoundingClientRect();
    let textBottom = 0;
    for (const word of node.querySelectorAll("span, a")) {
      if (!word.querySelector("svg") && word.textContent !== "") {
        textBottom = Math.max(textBottom, word.getBoundingClientRect().bottom);
      }
    }
    return {
      count: chevrons.length,
      fromEnd: first === undefined ? -1 : box.right - first.right,
      alone: first !== undefined && first.top >= textBottom - 1,
    };
  });
}

const lists = [
  { address: paths.groups, rows: "[data-part='list-editor.open']" },
  { address: paths.labels, rows: "[data-part='list-editor.open']" },
  { address: paths.issueTypes, rows: "[data-part='list-editor.open']" },
  { address: paths.relationKinds, rows: "[data-part='list-editor.open']" },
  { address: paths.projects, rows: "[data-part='list-editor.open']" },
  { address: paths.fields, rows: "li[data-part='item']" },
  { address: paths.roles, rows: "li[data-part='item']" },
  { address: paths.people, rows: "[data-part='people.person']" },
] as const;

test("ux_5 every row that opens an item has one chevron at its end", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await inOrder(
    lists.map((list) => async () => {
      await page.goto(list.address);
      const row = page.locator(list.rows).first();
      await expect(row).toBeVisible();
      const found = await chevronOf(row);
      expect(found.count, `${list.address}: chevrons in a row`).toBe(1);
      expect(found.fromEnd, `${list.address}: from the end of the row`).toBeLessThanOrEqual(24);
      expect(found.alone, `${list.address}: the chevron stands on a line of its own`).toBe(false);
    }),
  );
});

test("ux_5 the menus of the settings on a phone have a chevron at the end of each topic", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a phone only");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.organizationSettings);
  const topic = page.locator("[data-part='settings-frame.topics'] a").first();
  await expect(topic).toBeVisible();
  const found = await chevronOf(topic);
  expect(found.count).toBe(1);
  expect(found.fromEnd).toBeLessThanOrEqual(24);
});
