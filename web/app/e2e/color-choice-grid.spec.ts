import { t } from "@bugaboo/messages";
import { choiceColorWords } from "@bugaboo/api-client";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, the final pass: the color of a label is a set of small swatches in a grid that wraps, several in a row,
// not a long column of single swatches (Create the label stacked all twelve one per line).

test.use({ preparation: { run: prepareWeb } });

/** Half a pixel either way: boxes are measured in fractions. */
const ROUNDING = 0.5;

/** The swatches of the one color choice on the page. */
const swatchSelector = '[data-part="choice-group"] [data-part="field-swatch"]';

/** How many rows the swatches stand in, and how many stick out of the page at a side. It runs in the page. */
function placeOf({
  selector,
  rounding,
}: Readonly<{ selector: string; rounding: number }>): Readonly<{ rows: number; outside: number }> {
  const screen = document.documentElement.clientWidth;
  const tops = new Set<number>();
  let outside = 0;
  for (const node of document.querySelectorAll(selector)) {
    const box = node.getBoundingClientRect();
    tops.add(Math.round(box.top));
    outside += Number(box.left < -rounding || box.right > screen + rounding);
  }
  return { rows: tops.size, outside };
}

test("tag_02 the twelve colors of a new label stand several in a row, all in view", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.labels);
  const group = page.getByRole("group", { name: t("labels.field-new-color"), exact: true });
  await expect(group.getByRole("radio")).toHaveCount(choiceColorWords.length);
  const swatches = partIn(group, "field-swatch");
  await expect(swatches).toHaveCount(choiceColorWords.length);
  const placed = await page.evaluate(placeOf, { selector: swatchSelector, rounding: ROUNDING });
  // At least two in a row on a phone, more on a computer; none out of view at a side.
  expect(placed.rows).toBeLessThanOrEqual(choiceColorWords.length / 2);
  expect(placed.outside).toBe(0);
});

test("tag_02 the chosen color is marked by a tick as well as its ring", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.labels);
  const group = page.getByRole("group", { name: t("labels.field-new-color"), exact: true });
  await expect(partIn(group, "field-swatch.tick").filter({ visible: true })).toHaveCount(0);
  await group.getByRole("radio", { name: t("choice-color.cat3"), exact: true }).check();
  await expect(partIn(group, "field-swatch.tick").filter({ visible: true })).toHaveCount(1);
});
