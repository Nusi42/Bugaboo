import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, forgetChoices, makeAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { chooseGoHomeKeys, signIn } from "./support/flows.ts";
import { screenshotFolder, shot, shotWhole } from "./support/screenshots.ts";
import { paths } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

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

/** Takes the open screen in both looks. */
async function phoneScreens(page: Page): Promise<void> {
  await page.setViewportSize(viewports.phoneForPictures);
  await page.goto(paths.home);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await shot(page, "01-shell-phone", MILESTONE);
  await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
  await expect(page.getByRole("dialog", { name: t("shell.more") })).toBeVisible();
  await shot(page, "02-more-sheet-phone", MILESTONE);
  await page.keyboard.press("Escape");
  await page.goto(paths.mySettings);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.my-settings") })).toBeVisible();
  await shot(page, "03-my-settings-phone", MILESTONE);
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  await shot(page, "04-appearance-phone", MILESTONE);
  await page.goto(paths.shortcuts);
  await expect(page.getByRole("heading", { level: 1, name: t("shortcut.title") })).toBeVisible();
  await shot(page, "05-keyboard-shortcuts-phone", MILESTONE);
  await page.goto(`${paths.shortcuts}?edit=go-home`);
  await expect(page.getByRole("complementary", { name: t("shortcut.go-home") })).toBeVisible();
  await shot(page, "06-shortcut-editor-phone", MILESTONE);
}

async function quickScreens(page: Page): Promise<void> {
  await page.goto(paths.home);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await page.keyboard.press("Control+k");
  await page.getByRole("searchbox", { name: t("search.label"), exact: true }).fill("a");
  await expect(page.getByRole("option").first()).toBeVisible();
  await shot(page, "07-quick-search-phone", MILESTONE);
  await page.keyboard.press("Escape");
  await page.keyboard.press("?");
  await expect(page.getByRole("dialog", { name: t("shortcut.title") })).toBeVisible();
  await shot(page, "08-shortcut-list-phone", MILESTONE);
  await page.keyboard.press("Escape");
}

/** The pages of *My settings* and the organization's settings that WP-07 added, at phone width. */
async function settingsScreens(page: Page): Promise<void> {
  await page.setViewportSize(viewports.phoneForPictures);
  await page.goto(paths.region);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.region") })).toBeVisible();
  await shotWhole(page, "20-region-phone", MILESTONE);
  await page.goto(paths.startPage);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.start-page") })).toBeVisible();
  await shotWhole(page, "21-start-page-phone", MILESTONE);
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  await shotWhole(page, "22-appearance-samples-phone", MILESTONE);
  await page.goto(paths.signInRules);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.sign-in-rules") })).toBeVisible();
  await shotWhole(page, "23-sign-in-rules-phone", MILESTONE);
  // The administrator locks the theme; the page of a person then says why it cannot be changed.
  await page.goto(paths.themes);
  const row = page.getByRole("listitem").filter({
    has: page.getByRole("group", { name: t("theme.legend"), exact: true }),
  });
  await row.getByText(t("setting.ui.lock"), { exact: true }).click();
  await expect(row.getByRole("switch", { name: t("setting.ui.lock") })).toBeChecked();
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await shotWhole(page, "24-themes-lock-phone", MILESTONE);
  await page.goto(paths.appearance);
  await expect(page.getByText(t("setting.ui.locked"), { exact: true }).first()).toBeVisible();
  await shotWhole(page, "25-appearance-locked-phone", MILESTONE);
  await chooseGoHomeKeys(page);
  await page.goto(paths.shortcuts);
  await expect(page.getByRole("heading", { level: 1, name: t("shortcut.title") })).toBeVisible();
  await shotWhole(page, "26-keyboard-shortcuts-kept-phone", MILESTONE);
}

async function largerScreens(page: Page): Promise<void> {
  await page.setViewportSize(viewports.computer);
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  await shot(page, "12-shell-computer", MILESTONE);
  await page.setViewportSize(viewports.tablet);
  await page.goto(paths.appearance);
  await page.getByRole("button", { name: t("shell.menu-button"), exact: true }).click();
  await expect(page.getByRole("navigation", { name: t("shell.menu") })).toBeVisible();
  await shot(page, "15-menu-tablet", MILESTONE);
}

test("the screens of the app shell", async ({ page, server }) => {
  test.setTimeout(TAKING_ALL_MS);
  await forgetChoices(server, ada, true);
  await signIn(page, ada);
  await phoneScreens(page);
  await quickScreens(page);
  await settingsScreens(page);
  await largerScreens(page);
});
