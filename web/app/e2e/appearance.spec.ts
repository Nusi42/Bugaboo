import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, forgetChoices, makeAdmin } from "./support/accounts.ts";
import {
  expectNoOverlap,
  expectNoSidewaysScroll,
  expectNoViolations,
  themes,
  type Theme,
} from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});

test.beforeEach(async ({ server }) => {
  await forgetChoices(server);
});

/** Signs in and opens *My settings › Appearance*. */
async function openAppearance(page: Page): Promise<void> {
  await signIn(page, ada);
  await page.goto("/me/appearance");
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
}

/** Turns a switch on or off by pressing its words, as a person does. */
async function setSwitch(page: Page, name: string, on: boolean): Promise<void> {
  const field = page.getByRole("switch", { name, exact: true });
  if ((await field.isChecked()) !== on) {
    await page.getByText(name, { exact: true }).click();
  }
  await (on ? expect(field).toBeChecked() : expect(field).not.toBeChecked());
}

/** The value of a token on the page, as the browser works it out. */
function tokenValue(page: Page, name: string): Promise<string> {
  return page.evaluate(
    (token) => getComputedStyle(document.documentElement).getPropertyValue(token).trim(),
    name,
  );
}

/** The font size of the heading of the page, in pixels. */
function headingSize(page: Page): Promise<number> {
  return page.evaluate(() => {
    const heading = document.querySelector("h1");
    return heading === null ? 0 : Number(getComputedStyle(heading).fontSize.replace("px", ""));
  });
}

/**
 * Shows a theme the way a person does: *Always light* or *Always dark*, and the theme among the
 * samples of the light or the dark themes.
 */
async function pick(page: Page, theme: Theme): Promise<void> {
  const dark = theme.id === "dusk" || theme.id === "midnight";
  await page
    .getByRole("radio", {
      name: t(dark ? "appearance.mode.dark" : "appearance.mode.light"),
      exact: true,
    })
    .check();
  const group = page.getByRole("group", {
    name: t(dark ? "appearance.dark" : "appearance.light"),
    exact: true,
  });
  await group.getByRole("radio", { name: theme.label, exact: true }).check();
}

test("thm_01 each of the four themes can be chosen, and the choice is remembered", async ({
  page,
}) => {
  await openAppearance(page);
  await inOrder(
    themes.map((theme: Theme) => async () => {
      await pick(page, theme);
      await expect(page.locator("html")).toHaveAttribute("data-look", theme.id);
      // The choice is on the server before the page is loaded again.
      await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
      await page.reload();
      await expect(page.locator("html")).toHaveAttribute("data-look", theme.id);
    }),
  );
});

test("thm_02 Match my device uses the light choice on a light device and the dark choice on a dark one", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await openAppearance(page);
  await expect(page.locator("html")).toHaveAttribute("data-look", "daylight");
  const light = page.getByRole("group", { name: t("appearance.light") });
  await light.getByRole("radio", { name: t("theme.paper"), exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute("data-look", "paper");
});

test("thm_02 the dark choice shows on a dark device", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await openAppearance(page);
  await expect(page.locator("html")).toHaveAttribute("data-look", "dusk");
  const dark = page.getByRole("group", { name: t("appearance.dark") });
  await dark.getByRole("radio", { name: t("theme.midnight"), exact: true }).check();
  await expect(page.locator("html")).toHaveAttribute("data-look", "midnight");
  await page.emulateMedia({ colorScheme: "light" });
  await expect(page.locator("html")).toHaveAttribute("data-look", "daylight");
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-look", "midnight");
});

test("thm_06 Less motion turns motion off, and so does the setting of the device", async ({
  page,
}) => {
  await openAppearance(page);
  expect(await tokenValue(page, "--motion-base")).toBe(".2s");
  await setSwitch(page, t("appearance.less-motion"), true);
  await expect(page.locator("html")).toHaveAttribute("data-motion", "less");
  expect(await tokenValue(page, "--motion-base")).toBe("0s");
  await setSwitch(page, t("appearance.less-motion"), false);
  await expect(page.locator("html")).not.toHaveAttribute("data-motion", /./u);
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
  await expect(page.locator("html")).toHaveAttribute("data-spacing", "compact");
  expect(await tokenValue(page, "--space-4")).not.toBe(gap);
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-text", "larger");
  await expect(page.locator("html")).toHaveAttribute("data-spacing", "compact");
  await expectNoSidewaysScroll(page);
  await expectNoOverlap(page);
});

test("thm_06 more contrast gives stronger text and lines in every theme", async ({ page }) => {
  await openAppearance(page);
  await inOrder(
    themes.map((theme: Theme) => async () => {
      await pick(page, theme);
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
      await pick(page, theme);
      await expectNoViolations(page);
      await expectNoSidewaysScroll(page);
      await expectNoOverlap(page);
    }),
  );
});
