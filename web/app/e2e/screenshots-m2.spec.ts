import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, forgetChoices, makeAdmin } from "./support/accounts.ts";
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
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
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

/** The tallest a picture of one whole page may be. */
const TALLEST_PAGE = 8000;

/**
 * Takes the whole page in both looks. The content of the app scrolls inside the shell, so a
 * picture of the window shows only the first screenful: the window is made as tall as the
 * longest scrolling area needs, then put back.
 */
async function wholePage(page: Page, name: string): Promise<void> {
  const before = page.viewportSize() ?? sizes.phone;
  const missing = await page.evaluate(() => {
    let most = 0;
    for (const element of document.querySelectorAll("*")) {
      most = Math.max(most, element.scrollHeight - element.clientHeight);
    }
    return most;
  });
  await page.setViewportSize({
    width: before.width,
    height: Math.min(before.height + missing, TALLEST_PAGE),
  });
  await shot(page, name);
  await page.setViewportSize(before);
}

/** Gives *Go home* the keys `g i`, so the list of keys has one of the person's own to show. */
async function chooseGoHomeKeys(page: Page): Promise<void> {
  await page.goto("/me/shortcuts");
  await page.getByRole("button", { name: t("shortcut.go-home"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: t("shortcut.go-home"), exact: true });
  await panel.getByRole("textbox", { name: t("shortcuts.new-keys"), exact: true }).click();
  await page.keyboard.press("g");
  await page.keyboard.press("i");
  await panel.getByRole("button", { name: t("shortcuts.use") }).click();
  await expect(panel.getByText(t("shortcuts.saved"))).toBeVisible();
}

/** The pages of *My settings* and the organization's settings that WP-07 added, at phone width. */
async function settingsScreens(page: Page): Promise<void> {
  await page.setViewportSize(sizes.phone);
  await page.goto("/me/region");
  await expect(page.getByRole("heading", { level: 1, name: t("nav.region") })).toBeVisible();
  await wholePage(page, "20-region-phone");
  await page.goto("/me/start-page");
  await expect(page.getByRole("heading", { level: 1, name: t("nav.start-page") })).toBeVisible();
  await wholePage(page, "21-start-page-phone");
  await page.goto("/me/appearance");
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  await wholePage(page, "22-appearance-samples-phone");
  await page.goto("/settings/sign-in-rules");
  await expect(page.getByRole("heading", { level: 1, name: t("nav.sign-in-rules") })).toBeVisible();
  await wholePage(page, "23-sign-in-rules-phone");
  // The administrator locks the theme; the page of a person then says why it cannot be changed.
  await page.goto("/settings/themes");
  const row = page.getByRole("listitem").filter({
    has: page.getByRole("group", { name: t("theme.legend"), exact: true }),
  });
  await row.getByText(t("setting.ui.lock"), { exact: true }).click();
  await expect(row.getByRole("switch", { name: t("setting.ui.lock") })).toBeChecked();
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await wholePage(page, "24-themes-lock-phone");
  await page.goto("/me/appearance");
  await expect(page.getByText(t("setting.ui.locked"), { exact: true }).first()).toBeVisible();
  await wholePage(page, "25-appearance-locked-phone");
  await chooseGoHomeKeys(page);
  await page.goto("/me/shortcuts");
  await expect(page.getByRole("heading", { level: 1, name: t("shortcut.title") })).toBeVisible();
  await wholePage(page, "26-keyboard-shortcuts-kept-phone");
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

test("the screens of the app shell", async ({ page, server }) => {
  test.setTimeout(TAKING_ALL_MS);
  await forgetChoices(server, ada, true);
  await signIn(page, ada);
  await phoneScreens(page);
  await quickScreens(page);
  await frameScreens(page);
  await settingsScreens(page);
  await largerScreens(page);
});
