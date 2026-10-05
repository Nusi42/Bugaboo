import { t } from "@bugaboo/messages";

import { makeAdmin } from "./support/accounts.ts";
import {
  chooseTheme,
  expectNoSidewaysScroll,
  expectNoViolations,
  themes,
} from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});

for (const theme of themes) {
  test(`thm_01 the sign-in screen has no accessibility violations in ${theme.id}`, async ({
    page,
  }) => {
    await page.goto("/sign-in");
    await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
    await chooseTheme(page, theme);
    await expectNoViolations(page);
  });
}

test("thm_01 the sign-in screen does not scroll sideways", async ({ page }) => {
  await page.goto("/sign-in");
  await expectNoSidewaysScroll(page);
});

test("thm_02 Match my device follows the device and the choice is remembered", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await page.goto("/sign-in");
  await expect(page.locator("html")).toHaveAttribute("data-look", "dusk");
  await page.getByRole("radio", { name: t("theme.paper"), exact: true }).check();
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-look", "paper");
  await page.getByRole("radio", { name: t("theme.device") }).check();
  await expect(page.locator("html")).toHaveAttribute("data-look", "dusk");
});

test("thm_02 Match my device shows the resolved theme and follows the device live", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/sign-in");
  const device = page.getByRole("radio", { name: t("theme.device") });
  await expect(device).toBeChecked();
  await expect(page.locator("html")).toHaveAttribute("data-look", "daylight");
  await expect(device).toHaveAccessibleName(
    t("theme.device-resolved", { device: t("theme.device"), theme: t("theme.daylight") }),
  );
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-look", "dusk");
  await expect(device).toHaveAccessibleName(
    t("theme.device-resolved", { device: t("theme.device"), theme: t("theme.dusk") }),
  );
});

test("thm_02 Match my device names the theme of the device even when another is chosen", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/sign-in");
  await page.getByRole("radio", { name: t("theme.midnight"), exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute("data-look", "midnight");
  await expect(page.getByRole("radio", { name: t("theme.device") })).toHaveAccessibleName(
    t("theme.device-resolved", { device: t("theme.device"), theme: t("theme.daylight") }),
  );
});
