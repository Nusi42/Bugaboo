import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { expect, test } from "./support/fixtures.ts";
import { lowWindow, scrollingOf } from "./support/scrolling.ts";

// One way to scroll (UX §6.5) on the first start, which has its own server: it only exists while nobody has an
// account. The other pages before sign-in are in one-way-to-scroll.spec.ts.

test("ux_06_a_low_window_scrolls_the_card_of_the_setup_and_never_the_page", async ({ page }) => {
  await lowWindow(page);
  await page.goto(paths.setup);
  await expect(page.getByRole("heading", { level: 1, name: t("first-run.title") })).toBeVisible();
  expect(await scrollingOf(page)).toEqual({ page: 0, nested: [] });
});
