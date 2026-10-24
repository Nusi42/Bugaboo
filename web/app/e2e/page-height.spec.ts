import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { onThisLayout } from "./support/layout.ts";
import { paths } from "../src/shared/paths.ts";

// The page is as tall as the window (M4, the final check): the app scrolls inside its frame, so a
// switch or a choice far down the page must not make the page itself taller, with blank space under
// the bottom bar of a phone.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

async function expectPageAsTallAsWindow(page: Page): Promise<void> {
  const heights = await page.evaluate(() => ({
    page: document.documentElement.scrollHeight,
    window: window.innerHeight,
  }));
  expect(heights.page).toBeLessThanOrEqual(heights.window);
}

test("ux_01 a long page does not make the page itself taller than the window", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  // On a computer the page itself is as long as its content; the frame is the one of phone and tablet.
  await onThisLayout(page, {
    phone: expectPageAsTallAsWindow,
    tablet: expectPageAsTallAsWindow,
    computer: () => Promise.resolve(),
  });
});
