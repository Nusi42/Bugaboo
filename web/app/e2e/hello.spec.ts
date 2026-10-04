import { AxeBuilder } from "@axe-core/playwright";
import { t } from "@bugaboo/messages";
import { expect, test } from "@playwright/test";

const themes = [
  { id: "daylight", label: t("theme.daylight") },
  { id: "paper", label: t("theme.paper") },
  { id: "dusk", label: t("theme.dusk") },
  { id: "midnight", label: t("theme.midnight") },
] as const;

test.describe("thm_01 hello screen", () => {
  for (const theme of themes) {
    test(`has no accessibility violations in ${theme.id}`, async ({ page }) => {
      await page.goto("/");
      await expect(page.getByRole("heading", { name: t("hello.title") })).toBeVisible();
      await page.getByRole("radio", { name: theme.label, exact: true }).check();
      await expect(page.locator("html")).toHaveAttribute("data-look", theme.id);
      const results = await new AxeBuilder({ page }).analyze();
      expect(results.violations).toEqual([]);
    });
  }

  test("does not scroll sideways", async ({ page }) => {
    await page.goto("/");
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(overflow).toBeLessThanOrEqual(0);
  });

  test("Match my device follows the device and the choice is remembered", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("data-look", "dusk");
    await page.getByRole("radio", { name: t("theme.paper"), exact: true }).check();
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-look", "paper");
    await page.getByRole("radio", { name: t("theme.device") }).check();
    await expect(page.locator("html")).toHaveAttribute("data-look", "dusk");
  });

  test("Match my device shows the resolved theme and follows the device live", async ({ page }) => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/");
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
});
