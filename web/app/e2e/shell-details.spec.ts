import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { onThisLayout } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { paths } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

/** The menu of the app: the sidebar, or what a tablet slides in. */
function mainMenu(page: Page): Locator {
  return page.getByRole("navigation", { name: t("shell.menu"), exact: true });
}

/** Where the sidebar of a computer starts, in pixels from the top of the window. */
function topOf(page: Page): Promise<number> {
  return partIn(page, "sidebar").evaluate((node) => node.getBoundingClientRect().top);
}

async function stayingPut(page: Page): Promise<void> {
  // A short window and a long page: the page scrolls and the sidebar stays in the window, Sign out within its reach.
  await page.setViewportSize({ width: viewports.computer.width, height: 600 });
  await page.goto(paths.appearance);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  // The wheel over the page, not over the sidebar: the page scrolls, the sidebar stays at the top of the window.
  await page.mouse.move(viewports.computer.width - 100, 300);
  await page.mouse.wheel(0, 3000);
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0);
  const menu = mainMenu(page);
  await expect(menu).toBeInViewport();
  expect(Math.abs(await topOf(page))).toBeLessThanOrEqual(1);
  // Taller than the window (the sections of the menu say so where they are empty), it scrolls itself to Sign out.
  const leave = page.getByRole("button", { name: t("nav.sign-out"), exact: true });
  await leave.scrollIntoViewIfNeeded();
  await expect(leave).toBeInViewport();
  // A window so short that the sidebar does not fit scrolls the sidebar itself, not the page.
  await page.setViewportSize({ width: viewports.computer.width, height: 360 });
  await leave.scrollIntoViewIfNeeded();
  await expect(leave).toBeInViewport();
}

async function nothingToKeep(page: Page): Promise<void> {
  // A tablet and a phone have no sidebar to keep in the window.
  await expect(page.getByRole("button", { name: t("shell.narrow") })).toHaveCount(0);
}

test("ux_01 the sidebar of a computer stays in the window while the page scrolls", async ({
  page,
}) => {
  await signIn(page, ada);
  await onThisLayout(page, { computer: stayingPut, tablet: nothingToKeep, phone: nothingToKeep });
});

async function tooltipOnFocus(page: Page): Promise<void> {
  const menu = mainMenu(page);
  await menu.getByRole("button", { name: t("shell.narrow") }).click();
  // With icons only, the keyboard focus shows what the icon is.
  await menu.getByRole("button", { name: t("issues.new"), exact: true }).focus();
  await page.keyboard.press("Tab");
  await expect(menu.getByRole("link", { name: t("nav.home"), exact: true })).toBeFocused();
  await expect(page.getByRole("tooltip")).toHaveText(t("nav.home"));
}

async function noNarrowMenu(page: Page): Promise<void> {
  await expect(page.getByRole("button", { name: t("shell.narrow") })).toHaveCount(0);
}

test("ux_04 the narrow menu names an icon when it has the keyboard focus", async ({ page }) => {
  await signIn(page, ada);
  await onThisLayout(page, { computer: tooltipOnFocus, tablet: noNarrowMenu, phone: noNarrowMenu });
});

async function groupsOfTopics(page: Page): Promise<void> {
  await expect(page.getByRole("list", { name: t("nav.group.look"), exact: true })).toBeVisible();
  await expect(page.getByRole("list", { name: t("nav.group.sign-in"), exact: true })).toBeVisible();
}

async function groupsOnLargeScreens(page: Page): Promise<void> {
  await page.goto(paths.appearance);
  await groupsOfTopics(page);
}

async function groupsOnPhone(page: Page): Promise<void> {
  await page.goto(paths.mySettings);
  await groupsOfTopics(page);
}

test("ux_04 the groups of the settings menu are labelled lists", async ({ page }) => {
  await signIn(page, ada);
  await onThisLayout(page, {
    computer: groupsOnLargeScreens,
    tablet: groupsOnLargeScreens,
    phone: groupsOnPhone,
  });
});

async function openQuestion(page: Page): Promise<{ width: number; height: number }> {
  await page.goto(paths.sessions);
  await page.getByRole("button", { name: t("sessions.everywhere") }).click();
  const box = await page.getByRole("alertdialog").boundingBox();
  return { width: box?.width ?? 0, height: box?.height ?? 0 };
}

async function questionFillsThePhone(page: Page): Promise<void> {
  const box = await openQuestion(page);
  expect(box).toEqual({ width: page.viewportSize()?.width, height: page.viewportSize()?.height });
}

async function questionIsADialog(page: Page): Promise<void> {
  const box = await openQuestion(page);
  expect(box.width).toBeLessThan(page.viewportSize()?.width ?? 0);
}

test("ux_01 a question before something that cannot be undone fills the screen of a phone", async ({
  page,
}) => {
  await signIn(page, ada);
  await onThisLayout(page, {
    computer: questionIsADialog,
    tablet: questionIsADialog,
    phone: questionFillsThePhone,
  });
});

test("ux_01 the content is named for the router, so going back brings the page to where it was", async ({
  page,
}) => {
  await signIn(page, ada);
  await expect(page.locator("main")).toHaveAttribute("data-scroll-restoration-id", "content");
});
