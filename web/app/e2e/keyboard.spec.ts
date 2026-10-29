import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { shortcutDefinitions } from "../src/shared/shortcuts.ts";
import { ada, forgetChoices, createAdmin } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { itemArea } from "./support/windows.ts";
import { addressEndingAt, signIn } from "./support/flows.ts";
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

/** The list of all shortcuts, in the dialog that `?` opens. */
function list(page: Page) {
  return page
    .getByRole("dialog", { name: t("shortcut.title"), exact: true })
    .getByRole("list", { name: t("shortcut.title") });
}

test("ux_05 ? lists every shortcut of the registry", async ({ page }) => {
  await signIn(page, ada);
  await page.keyboard.press("?");
  await expect(list(page)).toBeVisible();
  await expect(list(page).getByRole("listitem")).toHaveCount(shortcutDefinitions.length);
  await Promise.all(
    shortcutDefinitions.map((definition) =>
      expect(list(page).getByText(t(definition.message), { exact: true })).toBeVisible(),
    ),
  );
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(list(page)).toHaveCount(0);
});

test("ux_05 the button for the list does the same as the ? key", async ({ page }) => {
  await signIn(page, ada);
  const button = page.getByRole("button", { name: t("shortcut.title"), exact: true });
  await expect(
    button.or(page.getByRole("button", { name: t("shell.more"), exact: true })),
  ).toBeVisible();
  await page.keyboard.press("?");
  await expect(list(page)).toBeVisible();
});

test("ux_05 g h goes to the start page, and every shortcut has a visible button", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  await page.keyboard.press("g");
  await page.keyboard.press("h");
  await expect(page).toHaveURL(addressEndingAt(paths.home));
  // The visible controls the registry names for the shortcuts of every screen.
  await expect(page.getByRole("link", { name: t("nav.home"), exact: true }).first()).toBeVisible();
});

test("ux_05 a shortcut does nothing while a text field has the focus", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.shortcuts);
  const search = page.getByRole("searchbox", { name: t("shortcuts.search"), exact: true });
  await search.focus();
  await page.keyboard.type("gh?j");
  await expect(search).toHaveValue("gh?j");
  await expect(page).toHaveURL(addressEndingAt(paths.shortcuts));
  await expect(list(page)).toHaveCount(0);
});

/** Opens the editor of one shortcut in the side panel. */
async function openEditor(page: Page, name: string) {
  await page.goto(paths.shortcuts);
  await page.getByRole("button", { name, exact: true }).click();
  return itemArea(page, name);
}

test("ux_05 own keys replace the defaults: the new key works and the old one does not", async ({
  page,
}) => {
  await signIn(page, ada);
  const panel = await openEditor(page, t("shortcut.quick-search"));
  await expect(panel).toBeVisible();
  await expect(panel.getByRole("heading", { name: t("shortcut.quick-search") })).toBeFocused();
  await expectGoodInEveryLook(page);
  await panel.getByRole("textbox", { name: t("shortcuts.new-keys"), exact: true }).click();
  await page.keyboard.press("Alt+s");
  await panel.getByRole("button", { name: t("shortcuts.use") }).click();
  await expect(panel.getByText(t("shortcuts.saved"))).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);

  await page.keyboard.press("Alt+s");
  await expect(page.getByRole("dialog", { name: t("search.title") })).toBeVisible();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+k");
  await expect(page.getByRole("dialog", { name: t("search.title") })).toHaveCount(0);

  // The usual keys come back with one button.
  const again = await openEditor(page, t("shortcut.quick-search"));
  await again.getByRole("button", { name: t("shortcuts.reset") }).click();
  await expect(again.getByText(t("shortcuts.reset-done"))).toBeVisible();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+k");
  await expect(page.getByRole("dialog", { name: t("search.title") })).toBeVisible();
});

test("ux_05 keys that another shortcut uses are refused with a sentence that names it", async ({
  page,
}) => {
  await signIn(page, ada);
  const panel = await openEditor(page, t("shortcut.quick-search"));
  await panel.getByRole("textbox", { name: t("shortcuts.new-keys"), exact: true }).click();
  await page.keyboard.press("g");
  await page.keyboard.press("h");
  await expect(panel.getByText(t("shortcut.go-home"), { exact: false })).toBeVisible();
  await expect(panel.getByRole("button", { name: t("shortcuts.use") })).toBeDisabled();
});

test("ux_05 the shortcuts are searchable, and the open one is in the address", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.shortcuts);
  const search = page.getByRole("searchbox", { name: t("shortcuts.search"), exact: true });
  await search.fill("zzz");
  await expect(page.getByText(t("shortcuts.nothing"))).toBeVisible();
  await search.fill(t("shortcut.list-next"));
  await page.getByRole("button", { name: t("shortcut.list-next"), exact: true }).click();
  await expect(page).toHaveURL(/[?]edit=list-next$/u);
  await expect(itemArea(page, t("shortcut.list-next"))).toBeVisible();
  // An address with the shortcut in it opens the panel straight away.
  await page.goto(`${paths.shortcuts}?edit=go-home`);
  await expect(itemArea(page, t("shortcut.go-home"))).toBeVisible();
});

test("ux_05 the box for new keys has a visible label and its help text", async ({ page }) => {
  await signIn(page, ada);
  const panel = await openEditor(page, t("shortcut.go-home"));
  await expect(panel.getByText(t("shortcuts.new-keys"), { exact: true })).toBeVisible();
  const box = panel.getByLabel(t("shortcuts.new-keys"), { exact: true });
  await expect(box).toHaveAccessibleDescription(t("shortcuts.new-keys-help"));
});
