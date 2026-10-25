import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { createAdmin, forgetChoices } from "./support/accounts.ts";
import { openAppearance, setSwitch } from "./support/appearance.ts";
import { expect, test } from "./support/fixtures.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { inEveryTheme } from "./support/shots-themes.ts";
import { paths } from "../src/shared/paths.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test.beforeEach(async ({ server }) => {
  await forgetChoices(server);
});

/** The width of the window of a small phone. */
const SMALL_PHONE_WIDTH = 360;

async function phoneBarFits(page: Page): Promise<void> {
  await page.setViewportSize({ width: SMALL_PHONE_WIDTH, height: 740 });
  await page.goto(paths.home);
  await expect(page.getByRole("navigation", { name: t("shell.menu"), exact: true })).toBeVisible();
  // Five tabs with Larger text must not make the page scroll sideways.
  const widths = await page.evaluate(() => ({
    page: document.documentElement.scrollWidth,
    window: window.innerWidth,
  }));
  expect(widths.page).toBeLessThanOrEqual(widths.window);
  await inEveryTheme(page, "13-phone-bar-larger-text");
}

test("ux_01 the bar of a phone fits 360 px with Larger text and the page does not scroll sideways", async ({
  page,
}) => {
  await openAppearance(page);
  await setSwitch(page, t("appearance.larger-text"), true);
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: phoneBarFits,
  });
});
