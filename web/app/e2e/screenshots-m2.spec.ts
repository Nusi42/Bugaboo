import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, makeAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder, shoot } from "./support/screenshots.ts";

// The screens of the app shell for the maintainer's review of M2, at phone width and, for the frames
// that only exist on larger screens, at the width of a tablet and a computer, each in a light and
// a dark look. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-m2.spec --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
/** All the screens, each in two looks, take longer than an ordinary test. */
const TAKING_ALL_MS = 300_000;
const MILESTONE = "M2";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});

/** A phone, a tablet and a computer. */
const sizes = {
  phone: { width: 400, height: 860 },
  tablet: { width: 900, height: 1000 },
  computer: { width: 1280, height: 800 },
} as const;

/** How long a sheet or a menu takes to slide in; the picture is taken when it has stopped. */
const SLIDING_MS = 500;

/** Takes the open screen in both looks. */
async function shot(page: Page, name: string): Promise<void> {
  await page.waitForTimeout(SLIDING_MS);
  await shoot(page, name, MILESTONE);
}

async function phoneScreens(page: Page): Promise<void> {
  await page.setViewportSize(sizes.phone);
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await shot(page, "01-shell-phone");
  await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
  await expect(page.getByRole("dialog", { name: t("shell.more") })).toBeVisible();
  await shot(page, "02-more-sheet-phone");
  await page.keyboard.press("Escape");
  await page.goto("/me");
  await expect(page.getByRole("heading", { level: 1, name: t("nav.my-settings") })).toBeVisible();
  await shot(page, "03-my-settings-phone");
  await page.goto("/me/appearance");
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  await shot(page, "04-appearance-phone");
  await page.goto("/me/shortcuts");
  await expect(page.getByRole("heading", { level: 1, name: t("shortcut.title") })).toBeVisible();
  await shot(page, "05-keyboard-shortcuts-phone");
  await page.goto("/me/shortcuts?edit=go-home");
  await expect(page.getByRole("complementary", { name: t("shortcut.go-home") })).toBeVisible();
  await shot(page, "06-shortcut-editor-phone");
}

async function quickScreens(page: Page): Promise<void> {
  await page.goto("/");
  await page.keyboard.press("Control+k");
  await page.getByRole("searchbox", { name: t("search.label"), exact: true }).fill("a");
  await expect(page.getByRole("option").first()).toBeVisible();
  await shot(page, "07-quick-search-phone");
  await page.keyboard.press("Escape");
  await page.keyboard.press("?");
  await expect(page.getByRole("dialog", { name: t("shortcut.title") })).toBeVisible();
  await shot(page, "08-shortcut-list-phone");
  await page.keyboard.press("Escape");
}

async function frameScreens(page: Page): Promise<void> {
  await page.goto("/frames?show=panel&item=first");
  await expect(page.getByRole("complementary", { name: t("frames.item-1") })).toBeVisible();
  await shot(page, "09-side-panel-phone");
  await page.goto("/frames?show=focus&item=second");
  await expect(page.getByRole("heading", { level: 2, name: t("frames.item-2") })).toBeVisible();
  await shot(page, "10-focus-view-phone");
  await page.goto("/frames");
  await page
    .getByRole("button", { name: t("frames.remove-name", { name: t("frames.item-3") }) })
    .click();
  await expect(page.getByRole("button", { name: t("common.undo") })).toBeVisible();
  await shot(page, "11-undo-phone");
}

async function largerScreens(page: Page): Promise<void> {
  await page.setViewportSize(sizes.computer);
  await page.goto("/me/appearance");
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  await shot(page, "12-shell-computer");
  await page.goto("/frames?show=focus&item=second");
  await expect(page.getByRole("heading", { level: 2, name: t("frames.item-2") })).toBeVisible();
  await shot(page, "13-focus-view-computer");
  await page.goto("/frames?show=panel&item=first");
  await expect(page.getByRole("complementary", { name: t("frames.item-1") })).toBeVisible();
  await shot(page, "14-side-panel-computer");
  await page.setViewportSize(sizes.tablet);
  await page.goto("/me/appearance");
  await page.getByRole("button", { name: t("shell.menu-button"), exact: true }).click();
  await expect(page.getByRole("navigation", { name: t("shell.menu") })).toBeVisible();
  await shot(page, "15-menu-tablet");
}

test("the screens of the app shell", async ({ page }) => {
  test.setTimeout(TAKING_ALL_MS);
  await signIn(page, ada);
  await phoneScreens(page);
  await quickScreens(page);
  await frameScreens(page);
  await largerScreens(page);
});
