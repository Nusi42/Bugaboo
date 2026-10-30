import { expectNoOverlap } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { cutWords, linesOfEach } from "./support/phone-list.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { boxOf } from "./support/windows.ts";
import { paths, projectIssuesAddress } from "../src/shared/paths.ts";

// M5 exit, package O: a row of a list on a phone that has only its name (a group, an issue type with its icon before the
// name, a kind of relation) is one line, and the line is in the middle of the row.

test.use({ sample: true });

test("ux_5 a row with only its name is one line in the middle of the row", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "the rows of a computer have their own look");
  await signIn(page, sampleAdministrator());
  await inOrder(
    [paths.groups, paths.issueTypes, paths.relationKinds].map((address) => async () => {
      await page.goto(address);
      const row = await boxOf(partIn(page, "list-editor.open").first());
      const name = await boxOf(partIn(page, "list-editor.name").first());
      const middle = Math.abs(name.y + name.height / 2 - (row.y + row.height / 2));
      expect(
        middle,
        `${address}: the name is level with the middle of its row`,
      ).toBeLessThanOrEqual(2);
    }),
  );
});

test("ux_4_2 the people of a row on a phone are written out on more lines, never cut with dots (M5 exit F-A3)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "the rows of a computer have a column of their own");
  await page.setViewportSize({ width: 360, height: 740 });
  await signIn(page, sampleAdministrator());
  // The rows of a project have the box to tick, so the least room; Niko Eriksen shares some issues with a second person.
  await page.goto(
    `${projectIssuesAddress("SHOP")}?s=${encodeURIComponent('assigned to is "Niko Eriksen"')}`,
  );
  const rows = page.locator("[data-part='list-row'][data-variant~='phone']");
  await expect(rows.first()).toBeVisible();
  const people = rows.locator("[data-part='list-row.cell'][data-variant~='person']");
  // Every line of people that is drawn shows all of its words, and the rows still stand one under the other.
  await expect(people.filter({ hasText: /\sand\s/u }).first()).toBeVisible();
  const cut = await cutWords(people);
  expect(cut).toEqual([]);
  await expectNoOverlap(page);
  // A line breaks only between two names, never inside one ("Niko / Eriksen"): each name is one piece on one line.
  const two = people.filter({ hasText: /\sand\s/u }).first();
  const names = partIn(two, "unbroken-word");
  await expect(names).toHaveCount(2);
  expect(await linesOfEach(names)).toEqual([1, 1]);
  // The date stands on the line of the last name.
  const [last, date] = await Promise.all([
    boxOf(names.last()),
    boxOf(partIn(two.locator(".."), "list-row.cell", "changed")),
  ]);
  expect(Math.abs(last.y + last.height - (date.y + date.height))).toBeLessThanOrEqual(3);
});

test("ux_4_2 on a computer a line of people too long for its column ends in dots, names cut like any words (review of F-A3)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "the rows of a computer have a column for the people");
  await signIn(page, sampleAdministrator());
  await page.goto(
    `${projectIssuesAddress("SHOP")}?s=${encodeURIComponent('assigned to is "Niko Eriksen"')}`,
  );
  const two = partIn(page, "list-row.cell", "person")
    .filter({ hasText: /\sand\s/u })
    .first();
  await expect(two).toBeVisible();
  // The line runs on past the column (so the cell draws its dots) instead of wrapping inside pieces that are clipped.
  await expect(two).toHaveCSS("text-overflow", "ellipsis");
  expect(await two.evaluate((cell) => cell.scrollWidth > cell.clientWidth)).toBe(true);
  // The dots of a cell are drawn only in text: a name kept whole as a box of its own would be clipped without them.
  await expect(partIn(two, "unbroken-word").first()).toHaveCSS("display", "inline");
});
