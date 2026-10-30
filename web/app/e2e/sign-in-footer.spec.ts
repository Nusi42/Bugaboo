import { expect, test } from "./support/fixtures.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O: the choice of theme at the foot of the pages before sign-in is no wider than the card above it.

test("ux_8 the footer of the sign-in page is no wider than its card", async ({ page }) => {
  await page.goto(paths.signIn);
  const card = partIn(page, "shell.narrow");
  await expect(card).toBeVisible();
  const footer = await boxOf(partIn(page, "shell.footer"));
  expect(footer.width).toBeLessThanOrEqual((await boxOf(card)).width + 1);
});

test("ux_8 the column of the sign-in page stands in the middle, with the normal gap above the theme", async ({
  page,
}) => {
  await page.goto(paths.signIn);
  const column = partIn(page, "shell.narrow");
  await expect(column).toBeVisible();
  const box = await boxOf(column);
  const width = await page.evaluate(() => window.innerWidth);
  // As much room on the left as on the right (a pixel of rounding).
  expect(Math.abs(box.x - (width - box.x - box.width))).toBeLessThanOrEqual(1);
  // The theme follows the card at the gap that is between the name of the product and the card.
  const header = await boxOf(partIn(page, "shell.header"));
  const footer = await boxOf(partIn(page, "shell.footer"));
  const above = box.y - (header.y + header.height);
  const below = footer.y - (box.y + box.height);
  expect(Math.abs(below - above)).toBeLessThanOrEqual(1);
});
