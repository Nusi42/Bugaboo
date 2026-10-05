import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, makeAdmin } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { onThisLayout } from "./support/layout.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});

/** Opens the frames page, which holds the empty frames the later screens open in. */
async function openFrames(page: Page, search = ""): Promise<void> {
  await signIn(page, ada);
  await page.goto(`/frames${search}`);
  await expect(page.getByRole("heading", { level: 1, name: t("frames.title") })).toBeVisible();
}

test("ux_01 a simple item opens in the side panel, Esc closes it and the focus goes back", async ({
  page,
}) => {
  await openFrames(page);
  const opener = page.getByRole("button", { name: t("frames.item-1"), exact: true });
  await opener.click();
  const panel = page.getByRole("complementary", { name: t("frames.item-1"), exact: true });
  await expect(panel).toBeVisible();
  await expect(panel.getByText(t("frames.panel-text"))).toBeVisible();
  await expect(page).toHaveURL(/show=panel&item=first$/u);
  await expect(panel.getByRole("heading", { name: t("frames.item-1") })).toBeFocused();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await expect(page).toHaveURL(/\/frames$/u);
  await expect(opener).toBeFocused();
});

async function listBesidePanel(page: Page): Promise<void> {
  await expect(page.getByRole("button", { name: t("frames.item-1"), exact: true })).toBeVisible();
}

async function panelAlone(page: Page): Promise<void> {
  await expect(page.getByRole("button", { name: t("frames.item-1"), exact: true })).toHaveCount(0);
}

test("ux_01 the side panel has a button that closes it; the list stays in view, except on a phone", async ({
  page,
}) => {
  await openFrames(page, "?show=panel&item=second");
  const panel = page.getByRole("complementary", { name: t("frames.item-2"), exact: true });
  await expect(panel).toBeVisible();
  await onThisLayout(page, {
    computer: listBesidePanel,
    tablet: listBesidePanel,
    phone: panelAlone,
  });
  await panel.getByRole("button", { name: t("panel.close") }).click();
  await expect(panel).toHaveCount(0);
  await expect(page.getByRole("button", { name: t("frames.item-1"), exact: true })).toBeVisible();
});

/** The focus view of the second example is open. */
async function openFocusView(page: Page): Promise<void> {
  await page
    .getByRole("button", { name: t("frames.open-focus", { name: t("frames.item-2") }) })
    .click();
  await expect(page.getByRole("heading", { level: 2, name: t("frames.item-2") })).toBeVisible();
}

test("ux_01 the focus view gives the item the area; j and k, Esc and the buttons move around", async ({
  page,
}) => {
  await openFrames(page);
  await openFocusView(page);
  await expect(page.getByText(t("frames.focus-text"))).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("j");
  await expect(page.getByRole("heading", { level: 2, name: t("frames.item-3") })).toBeVisible();
  await page.keyboard.press("k");
  await expect(page.getByRole("heading", { level: 2, name: t("frames.item-2") })).toBeVisible();
  await page.getByRole("button", { name: t("list.next"), exact: true }).click();
  await expect(page.getByRole("heading", { level: 2, name: t("frames.item-3") })).toBeVisible();
  await page.getByRole("button", { name: t("list.previous"), exact: true }).click();
  await expect(page.getByRole("heading", { level: 2, name: t("frames.item-2") })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("heading", { level: 2, name: t("frames.item-2") })).toHaveCount(0);
  await expect(page.getByRole("button", { name: t("frames.item-1"), exact: true })).toBeVisible();
});

async function listCanBeHidden(page: Page): Promise<void> {
  const slim = page.getByRole("list", { name: t("frames.list") });
  await expect(slim.getByRole("button", { name: t("frames.item-1"), exact: true })).toBeVisible();
  await page.getByRole("button", { name: t("focus.hide-list") }).click();
  await expect(slim).toHaveCount(0);
  await page.getByRole("button", { name: t("focus.show-list") }).click();
  await expect(slim).toBeVisible();
}

async function noListOnAPhone(page: Page): Promise<void> {
  await expect(page.getByRole("button", { name: t("focus.hide-list") })).toHaveCount(0);
  await expect(page.getByRole("list", { name: t("frames.list") })).toHaveCount(0);
}

test("ux_01 the focus view keeps a slim list, which can be hidden; a phone shows the item alone", async ({
  page,
}) => {
  await openFrames(page);
  await openFocusView(page);
  await onThisLayout(page, {
    computer: listCanBeHidden,
    tablet: listCanBeHidden,
    phone: noListOnAPhone,
  });
});

async function sidebarIsNarrow(page: Page): Promise<void> {
  const menu = page.getByRole("navigation", { name: t("shell.menu"), exact: true });
  await expect(menu.getByRole("button", { name: t("shell.widen") })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(menu.getByRole("button", { name: t("shell.narrow") })).toBeVisible();
}

async function nothingToNarrow(page: Page): Promise<void> {
  await page.keyboard.press("Escape");
  await expect(page.getByRole("button", { name: t("frames.item-1"), exact: true })).toBeVisible();
}

test("ux_01 the sidebar shrinks to icons while the focus view is open", async ({ page }) => {
  await openFrames(page);
  await openFocusView(page);
  await onThisLayout(page, {
    computer: sidebarIsNarrow,
    tablet: nothingToNarrow,
    phone: nothingToNarrow,
  });
});

test("ux_06 removing is done at once and Undo brings it back", async ({ page }) => {
  await openFrames(page);
  const row = page.getByRole("button", { name: t("frames.item-1"), exact: true });
  await page
    .getByRole("button", { name: t("frames.remove-name", { name: t("frames.item-1") }) })
    .click();
  await expect(row).toHaveCount(0);
  const offer = page
    .getByRole("status")
    .filter({ hasText: t("frames.removed", { name: t("frames.item-1") }) });
  await expect(offer).toBeVisible();
  await expectGoodInEveryLook(page);
  await offer.getByRole("button", { name: t("common.undo") }).click();
  await expect(row).toBeVisible();
  await expect(offer).toHaveCount(0);
});

test("ux_06 Undo is on offer for 10 seconds and not longer", async ({ page }) => {
  await page.clock.install();
  await openFrames(page);
  await page
    .getByRole("button", { name: t("frames.remove-name", { name: t("frames.item-3") }) })
    .click();
  const undo = page.getByRole("button", { name: t("common.undo") });
  await expect(undo).toBeVisible();
  await page.clock.fastForward(9000);
  await expect(undo).toBeVisible();
  await page.clock.fastForward(1500);
  await expect(undo).toHaveCount(0);
  await expect(page.getByRole("button", { name: t("frames.item-3"), exact: true })).toHaveCount(0);
});
