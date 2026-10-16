import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada, forgetChoices } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { newIssueDialog as dialog, openNewIssueDialog, prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { pixelsOf } from "./support/tokens.ts";
import {
  boxOf,
  columns,
  dragEastEdge,
  footerButtons,
  keyboardStep,
  menuStep,
  openWindowReady,
  pageSize,
  pickSizeStep,
  pressSizeKey,
  scrollFormToEnd,
  titleBox,
  windowBox,
  windowButton,
  windowDock,
  windowEdges,
  windowSize,
} from "./support/windows.ts";

// The window of the new-issue form (UX §4.5, §6.3, R10, M3 WP-17): minimize and back, maximize and
// restore, resizing with the pointer, the keys and the size menu, the size that is remembered, the
// form that follows the width of its own window, and the draft that nothing here loses. A phone
// has a full screen with Back instead, so these tests are for the computer.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });
test.beforeEach(async ({ page, server }) => {
  test.skip(layoutOf(page) !== "computer", "a phone has a full screen, not a window");
  // The size a test gave the window is the person's setting: the next test starts from the usual.
  await forgetChoices(server);
});

/** The window is open on the issues of the project WEB, with its fields. */
async function openIt(page: Parameters<typeof signIn>[0]): Promise<void> {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openWindowReady(page);
}

test("ux_6_3 minimize parks the window as a bar with the focus, and the bar opens it with the text", async ({
  page,
}) => {
  await openIt(page);
  await titleBox(page).fill("Typed before minimizing");
  await windowButton(page, t("window.minimize")).click();
  await expect(dialog(page)).toHaveCount(0);
  await expect(windowDock(page)).toBeVisible();
  await expect(windowDock(page)).toContainText(t("window.draft-kept"));
  // The window that had the focus is gone: the bar has it.
  await expect(windowDock(page)).toBeFocused();
  await windowDock(page).click();
  await expect(dialog(page)).toBeVisible();
  await expect(windowDock(page)).toHaveCount(0);
  await expect(titleBox(page)).toHaveValue("Typed before minimizing");
});

test("ux_6_3 a minimized window keeps its draft in the store, also over a reload", async ({
  page,
}) => {
  await openIt(page);
  await titleBox(page).fill("Kept while minimized");
  await windowButton(page, t("window.minimize")).click();
  await expect(windowDock(page)).toBeVisible();
  // The server keeps a draft a moment after the last key; the browser keeps it at once.
  await page.reload();
  await openNewIssueDialog(page);
  await dialog(page)
    .getByRole("button", { name: /^Continue your draft from/u })
    .click();
  await expect(titleBox(page)).toHaveValue("Kept while minimized");
});

test("ux_6_3 maximize fills the page, and restore down gives the size back", async ({ page }) => {
  await openIt(page);
  const before = await windowSize(page);
  await windowButton(page, t("window.maximize")).click();
  await expect.poll(() => windowSize(page)).toEqual(pageSize(page));
  // A maximized window is not resized, and the button says what it does now.
  await expect(partIn(page, "window.handle")).toHaveCount(0);
  await windowButton(page, t("window.restore-down")).click();
  await expect.poll(() => windowSize(page)).toEqual(before);
});

test("ux_6_3 a window that closes opens again as it was first drawn", async ({ page }) => {
  await openIt(page);
  const before = await windowSize(page);
  await windowButton(page, t("window.maximize")).click();
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  await openWindowReady(page);
  expect(await windowSize(page)).toEqual(before);
  await expect(windowButton(page, t("window.maximize"))).toBeVisible();
});

test("ux_04 Ctrl and Alt with the arrow keys resize the window, and the size is remembered after a reload", async ({
  page,
}) => {
  await openIt(page);
  const before = await windowSize(page);
  await pressSizeKey(page, "ArrowRight");
  await expect
    .poll(() => windowSize(page))
    .toEqual({
      width: before.width + keyboardStep,
      height: before.height,
    });
  // The window is as tall as the page lets it be, so a step up makes it shorter.
  await pressSizeKey(page, "ArrowUp");
  await expect
    .poll(() => windowSize(page))
    .toEqual({
      width: before.width + keyboardStep,
      height: before.height - keyboardStep,
    });
  const after = await windowSize(page);
  await page.reload();
  await openWindowReady(page);
  await expect.poll(() => windowSize(page)).toEqual(after);
});

test("ux_04 a person who cannot drag changes the size with the size menu", async ({ page }) => {
  await openIt(page);
  const before = await windowSize(page);
  await pickSizeStep(page, "wider");
  await expect
    .poll(() => windowSize(page))
    .toEqual({
      width: before.width + menuStep,
      height: before.height,
    });
  await pickSizeStep(page, "shorter");
  await expect
    .poll(() => windowSize(page))
    .toEqual({
      width: before.width + menuStep,
      height: before.height - menuStep,
    });
  // The menu gives the focus back to its button, and the size is remembered like any other.
  await expect(windowButton(page, t("window.size-menu"))).toBeFocused();
  const after = await windowSize(page);
  await page.reload();
  await openWindowReady(page);
  await expect.poll(() => windowSize(page)).toEqual(after);
});

test("ux_6_3 the pointer resizes the window at an edge, and the window stays in the middle", async ({
  page,
}) => {
  await openIt(page);
  const before = await windowSize(page);
  const box = await boxOf(partIn(page, "window.handle", "e"));
  const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + 40, y, { steps: 4 });
  await page.mouse.up();
  await expect
    .poll(() => windowSize(page))
    .toEqual({
      width: before.width + 80,
      height: before.height,
    });
  // All eight places to drag are there while the window can be resized.
  await Promise.all(
    windowEdges.map((edge) => expect(partIn(page, "window.handle", edge)).toHaveCount(1)),
  );
});

test("ux_6_3 the form follows the width of its window: one column below 880, the fields at the side from 880, the text centered from 1200", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  // The widths of the window that change the form are tokens of the theme file.
  const side = await pixelsOf(page, "--bp-window-side");
  const wide = await pixelsOf(page, "--bp-window-wide");
  await page.setViewportSize({ width: side - 20, height: 800 });
  await openWindowReady(page);
  await windowButton(page, t("window.maximize")).click();
  await expect.poll(() => windowSize(page)).toEqual({ width: side - 20, height: 800 });
  const narrow = await columns(page);
  expect(narrow.fields.y).toBeGreaterThanOrEqual(narrow.text.y + narrow.text.height);
  await page.setViewportSize({ width: side + 20, height: 800 });
  await expect
    .poll(async () => (await columns(page)).fields.x)
    .toBeGreaterThan((await columns(page)).text.x + 300);
  const sideBySide = await columns(page);
  expect(sideBySide.fields.y).toBeLessThan(sideBySide.text.y + sideBySide.text.height);
  // Below the wide width the text starts at the left edge of the form.
  expect(sideBySide.text.x - sideBySide.modal.x).toBeLessThan(40);
  await page.setViewportSize({ width: wide + 200, height: 800 });
  await expect
    .poll(async () => (await columns(page)).text.x - (await columns(page)).modal.x)
    .toBeGreaterThan(100);
});

test("ux_6_3 the footer has two rows: the hint, then the buttons in one row with Create last, also at the smallest size", async ({
  page,
}) => {
  await openIt(page);
  const usual = await footerButtons(page);
  expect(usual.create.y).toBeGreaterThan(usual.note.y);
  expect(Math.abs(usual.create.y - usual.cancel.y)).toBeLessThanOrEqual(2);
  expect(Math.abs(usual.create.y - usual.another.y)).toBeLessThanOrEqual(2);
  expect(usual.create.x).toBeGreaterThan(usual.another.x);
  expect(usual.another.x).toBeGreaterThan(usual.cancel.x);
  // As small as it goes: Create is still not alone on a row, and the row ends at the right.
  await dragEastEdge(page, -400);
  const smallest = await pixelsOf(page, "--size-window-width-min");
  await expect.poll(async () => (await windowSize(page)).width).toBeLessThanOrEqual(smallest);
  const small = await footerButtons(page);
  expect(Math.abs(small.create.y - small.cancel.y)).toBeLessThanOrEqual(2);
  const window = await boxOf(windowBox(page));
  expect(window.x + window.width - (small.create.x + small.create.width)).toBeLessThan(40);
});

test("ux_6_3 a minimized window stands after the menu: Sign out stays clickable", async ({
  page,
}) => {
  await openIt(page);
  await windowButton(page, t("window.minimize")).click();
  await expect(windowDock(page)).toBeVisible();
  const dock = await boxOf(windowDock(page));
  const signOut = await boxOf(page.getByRole("button", { name: t("nav.sign-out"), exact: true }));
  // The bar starts to the right of the menu and does not touch the button.
  expect(dock.x).toBeGreaterThanOrEqual(signOut.x + signOut.width);
  await page.getByRole("button", { name: t("nav.sign-out"), exact: true }).click();
  await expect(page.getByRole("button", { name: t("sign-in.submit"), exact: true })).toBeVisible();
});

test("ux_6_3 Esc closes the window and keeps the draft", async ({ page }) => {
  await openIt(page);
  await titleBox(page).fill("Kept by Esc");
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  await expect(windowDock(page)).toHaveCount(0);
  await openNewIssueDialog(page);
  await dialog(page)
    .getByRole("button", { name: /^Continue your draft from/u })
    .click();
  await expect(titleBox(page)).toHaveValue("Kept by Esc");
});

test("ux_6_3 the title bar and the footer stay in place while the form scrolls", async ({
  page,
}) => {
  await openIt(page);
  const footer = partIn(page, "window.footer");
  const before = await boxOf(footer);
  await scrollFormToEnd(page);
  expect(await boxOf(footer)).toEqual(before);
  // The main button is the last one, at the right of the footer (R6).
  const create = dialog(page).getByRole("button", { name: t("issues.create"), exact: true });
  const cancel = dialog(page).getByRole("button", { name: t("common.cancel"), exact: true });
  expect((await boxOf(create)).x).toBeGreaterThan((await boxOf(cancel)).x);
});
