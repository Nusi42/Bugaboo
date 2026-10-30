import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, cleo, createAdmin, createPerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { nothingFound } from "./support/quick-search.ts";
import { boxOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await (await createPerson(server, admin, cleo)).dispose();
      await admin.dispose();
    },
  },
});

/** The dialog of quick search. */
function dialog(page: Page) {
  return page.getByRole("dialog", { name: t("search.title"), exact: true });
}

/** The box of quick search. */
function box(page: Page) {
  return dialog(page).getByRole("searchbox", { name: t("search.label"), exact: true });
}

test("cfg_08 Ctrl+K opens quick search, finds a page and goes there", async ({ page }) => {
  await signIn(page, ada);
  await page.keyboard.press("Control+k");
  await expect(dialog(page)).toBeVisible();
  await expect(box(page)).toBeFocused();
  await box(page).fill("appear");
  const found = dialog(page).getByRole("option", { name: new RegExp(t("nav.appearance"), "u") });
  await expect(found).toBeVisible();
  await expect(found).toContainText(t("nav.my-settings"));
  await expectGoodInEveryLook(page);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(addressEndingAt(paths.appearance));
  await expect(dialog(page)).toHaveCount(0);
  // The focus is on the heading of the page that opened, not left behind in the closed dialog.
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeFocused();
});

test("cfg_08 Esc closes quick search and the focus goes back where it was", async ({ page }) => {
  await signIn(page, ada);
  const home = page.getByRole("link", { name: t("nav.home"), exact: true }).first();
  await home.focus();
  await page.keyboard.press("Control+k");
  await expect(dialog(page)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  await expect(home).toBeFocused();
  // The box starts empty every time, and the slash is kept for the search bar of a later version.
  await page.keyboard.press("/");
  await expect(dialog(page)).toHaveCount(0);
  await page.keyboard.press("Control+k");
  await expect(box(page)).toHaveValue("");
  await page.keyboard.press("Escape");
});

test("cfg_08 the search button opens it too, and a word with no match says so", async ({
  page,
}) => {
  await signIn(page, ada);
  await page
    .getByRole("button", { name: t("shell.search") })
    .first()
    .click();
  await expect(dialog(page)).toBeVisible();
  await expect(
    dialog(page).getByRole("option", { name: new RegExp(t("nav.home"), "u") }),
  ).toBeVisible();
  await box(page).fill("zzzzzz");
  await expect(nothingFound(page)).toBeVisible();
  await dialog(page)
    .getByRole("button", { name: t("common.close") })
    .click();
  await expect(dialog(page)).toHaveCount(0);
});

test("cfg_08 quick search lists only the pages the person may open", async ({ page }) => {
  await signIn(page, cleo);
  await page.keyboard.press("Control+k");
  await box(page).fill(t("nav.people"));
  await expect(nothingFound(page)).toBeVisible();
  await box(page).fill(t("nav.appearance"));
  await expect(
    dialog(page).getByRole("option", { name: new RegExp(t("nav.appearance"), "u") }),
  ).toBeVisible();
});

test("cfg_08 Ctrl+K opens quick search also while a text field has the focus", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.shortcuts);
  const search = page.getByRole("searchbox", { name: t("shortcuts.search"), exact: true });
  await search.focus();
  await page.keyboard.press("Control+k");
  await expect(dialog(page)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(search).toBeFocused();
});

test("cfg_08 Esc closes quick search at once, also with a word typed, and the keys work again (M5 exit)", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.keyboard.press("Control+k");
  await page.getByRole("searchbox", { name: t("search.label"), exact: true }).fill("a");
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  // Focus is back on the page, not in a text box: the key for the list of shortcuts works.
  await page.keyboard.press("?");
  await expect(page.getByRole("dialog", { name: t("shortcut.title") })).toBeVisible();
});

test("cfg_08 the sidebar says Quick search for the overlay, and the Search page is the one item highlighted on it (L19)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer has the sidebar");
  await signIn(page, ada);
  await page.goto(paths.search);
  const quick = page.getByRole("button", { name: new RegExp(`^${t("shell.quick-search")}`, "u") });
  await expect(quick).toBeVisible();
  await expect(quick).not.toHaveAttribute("aria-current", /.+/u);
  await expect(
    page.getByRole("link", { name: t("search.page.title"), exact: true }),
  ).toHaveAttribute("aria-current", "page");
});

test("cfg_08 on a phone a long text in quick search keeps its end in view, beside the clear mark (M5 exit F-A7)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "the box of a phone is the narrow one");
  await signIn(page, ada);
  await page
    .getByRole("navigation", { name: t("shell.menu"), exact: true })
    .getByRole("button", { name: t("shell.search"), exact: true })
    .click();
  await box(page).fill("Image gallery does not save the name of the picture");
  const clear = dialog(page).getByRole("button", { name: t("search.clear"), exact: true });
  const [field, mark] = await Promise.all([boxOf(box(page)), boxOf(clear)]);
  // The words end before the clear mark starts, and the end of the text is what the box shows.
  expect(field.x + field.width).toBeLessThanOrEqual(mark.x);
  await expect
    .poll(() =>
      box(page).evaluate((input) => input.scrollLeft + input.clientWidth >= input.scrollWidth - 1),
    )
    .toBe(true);
});

test("cfg_08 a result of one line stands in the middle of its row, as every row of quick search (M2 07)", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.home);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await page.keyboard.press("Control+k");
  await box(page).fill("a");
  await expect(dialog(page).getByRole("option").first()).toBeVisible();
  // Every result: the room above its words is the room below them (a row of one line and of two lines alike).
  const rooms = await dialog(page)
    .getByRole("option")
    .evaluateAll((options: readonly HTMLElement[]) => {
      const found: number[] = [];
      for (const option of options) {
        const row = option.getBoundingClientRect();
        let top = row.bottom;
        let bottom = row.top;
        for (const child of option.children) {
          const words = child.getBoundingClientRect();
          top = Math.min(top, words.top);
          bottom = Math.max(bottom, words.bottom);
        }
        found.push(Math.abs(top - row.top - (row.bottom - bottom)));
      }
      return found;
    });
  expect(rooms.length).toBeGreaterThan(1);
  expect(Math.max(...rooms)).toBeLessThanOrEqual(1);
});
