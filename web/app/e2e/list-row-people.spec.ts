import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// The people of a row of the issue list (UX §4.2, M5 exit F-A3 and its review): on a computer a line too long for its
// column ends in the dots of the cell; on a phone the names wrap whole (list-rows-phone.spec.ts).

test.use({ sample: true });

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
