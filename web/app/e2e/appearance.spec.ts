import { t } from "@bugaboo/messages";
import { tokenValue } from "./support/tokens.ts";
import { type Page } from "@playwright/test";

import { forgetChoices, createAdmin } from "./support/accounts.ts";
import {
  expectNoOverlap,
  expectNoSidewaysScroll,
  expectNoViolations,
  pickTheme,
  themes,
  type Theme,
  expectLook,
} from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openAppearance, setSwitch } from "./support/appearance.ts";
import { inOrder } from "./support/sequence.ts";
import { variants } from "../../packages/ui/src/tokens.ts";

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

/** The font size of the heading of the page, in pixels. */
function headingSize(page: Page): Promise<number> {
  return page.evaluate(() => {
    const heading = document.querySelector("h1");
    return heading === null ? 0 : Number(getComputedStyle(heading).fontSize.replace("px", ""));
  });
}

test("thm_01 each of the four themes can be chosen, and the choice is remembered", async ({
  page,
}) => {
  await openAppearance(page);
  await inOrder(
    themes.map((theme: Theme) => async () => {
      await pickTheme(page, theme);
      await expectLook(page, theme.id);
      // The choice is on the server before the page is loaded again.
      await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
      await page.reload();
      await expectLook(page, theme.id);
    }),
  );
});

test("thm_02 Match my device uses the light choice on a light device and the dark choice on a dark one", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await openAppearance(page);
  await expectLook(page, "daylight");
  const light = page.getByRole("group", { name: t("appearance.light") });
  await light.getByRole("radio", { name: t("theme.paper"), exact: true }).check();
  await expectLook(page, "paper");
});

test("thm_02 the dark choice shows on a dark device", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await openAppearance(page);
  await expectLook(page, "dusk");
  const dark = page.getByRole("group", { name: t("appearance.dark") });
  await dark.getByRole("radio", { name: t("theme.midnight"), exact: true }).check();
  await expectLook(page, "midnight");
  await page.emulateMedia({ colorScheme: "light" });
  await expectLook(page, "daylight");
  await page.emulateMedia({ colorScheme: "dark" });
  await expectLook(page, "midnight");
});

test("thm_06 Less motion turns motion off, and so does the setting of the device", async ({
  page,
}) => {
  await openAppearance(page);
  expect(await tokenValue(page, "--motion-base")).toBe(".2s");
  await setSwitch(page, t("appearance.less-motion"), true);
  await expect(page.locator("html")).toHaveAttribute(
    variants["less-motion"].attribute,
    variants["less-motion"].value,
  );
  expect(await tokenValue(page, "--motion-base")).toBe("0s");
  await setSwitch(page, t("appearance.less-motion"), false);
  await expect(page.locator("html")).not.toHaveAttribute(variants["less-motion"].attribute, /./u);
  await page.emulateMedia({ reducedMotion: "reduce" });
  expect(await tokenValue(page, "--motion-base")).toBe("0s");
});

test("thm_06 Larger text makes the text bigger and Compact makes the spacing smaller", async ({
  page,
}) => {
  await openAppearance(page);
  const normal = await headingSize(page);
  const gap = await tokenValue(page, "--space-4");
  await setSwitch(page, t("appearance.larger-text"), true);
  await expect.poll(() => headingSize(page)).toBeGreaterThan(normal);
  await page.getByRole("radio", { name: t("appearance.spacing.compact"), exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute(
    variants["compact"].attribute,
    variants["compact"].value,
  );
  expect(await tokenValue(page, "--space-4")).not.toBe(gap);
  // Both choices are on the server before the page is loaded again.
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute(
    variants["larger-text"].attribute,
    variants["larger-text"].value,
  );
  await expect(page.locator("html")).toHaveAttribute(
    variants["compact"].attribute,
    variants["compact"].value,
  );
  await expectNoSidewaysScroll(page);
  await expectNoOverlap(page);
});

test("thm_06 more contrast gives stronger text and lines in every theme", async ({ page }) => {
  await openAppearance(page);
  await inOrder(
    themes.map((theme: Theme) => async () => {
      await pickTheme(page, theme);
      await page.emulateMedia({ contrast: "no-preference" });
      const usual = await tokenValue(page, "--fg-muted");
      await page.emulateMedia({ contrast: "more" });
      await expect.poll(() => tokenValue(page, "--fg-muted")).not.toBe(usual);
      await expectNoViolations(page);
      await expectNoSidewaysScroll(page);
    }),
  );
});

test("thm_01 every theme is free of accessibility problems with larger text and compact spacing", async ({
  page,
}) => {
  await openAppearance(page);
  await setSwitch(page, t("appearance.larger-text"), true);
  await page.getByRole("radio", { name: t("appearance.spacing.compact"), exact: true }).check();
  await inOrder(
    themes.map((theme: Theme) => async () => {
      await pickTheme(page, theme);
      await expectNoViolations(page);
      await expectNoSidewaysScroll(page);
      await expectNoOverlap(page);
    }),
  );
});
