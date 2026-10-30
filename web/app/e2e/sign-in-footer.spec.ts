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
