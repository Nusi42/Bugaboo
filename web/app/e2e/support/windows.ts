import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { part } from "../../../packages/ui/src/tokens.ts";
import { keyboardStep, menuStep, windowEdges } from "../../../packages/ui/src/window-size.ts";
import { newIssueDialog, openNewIssueDialog, titleLabel } from "./issues.ts";

export { keyboardStep, menuStep, windowEdges };

/** An element of the app by the part it draws, as the theme file names it. */
export function partIn(scope: Locator | Page, name: string, variant?: string): Locator {
  const variantSelector = variant === undefined ? "" : `[data-variant="${variant}"]`;
  return scope.locator(`[data-part="${name}"]${variantSelector}`);
}

/** The box of the window: the part that the size variables are set on. */
export function windowBox(page: Page): Locator {
  return partIn(page, part.overlayModal, "window");
}

/** The bar a minimized window becomes. */
export function windowDock(page: Page): Locator {
  return partIn(page, part.windowDock);
}

/** The button of the title bar of the open window. */
export function windowButton(page: Page, name: string): Locator {
  return newIssueDialog(page).getByRole("button", { name, exact: true });
}

/** How big the window is on the page right now. */
export async function windowSize(page: Page): Promise<{ width: number; height: number }> {
  const box = await boxOf(windowBox(page));
  return { width: Math.round(box.width), height: Math.round(box.height) };
}

/** Opens the new-issue window and waits until its fields are there, so its size is settled. */
export async function openWindowReady(page: Page): Promise<void> {
  await openNewIssueDialog(page);
  await expect(newIssueDialog(page).getByLabel(titleLabel(), { exact: true })).toBeVisible();
}

/** The title box of the window. */
export function titleBox(page: Page): Locator {
  return newIssueDialog(page).getByLabel(titleLabel(), { exact: true });
}

/** Presses the size keys: Ctrl and Alt with an arrow key, inside the window. */
export async function pressSizeKey(page: Page, arrow: string): Promise<void> {
  await titleBox(page).focus();
  await page.keyboard.press(`Control+Alt+${arrow}`);
}

/** Opens the size menu of the title bar and picks a step. */
export async function pickSizeStep(page: Page, step: "wider" | "narrower" | "taller" | "shorter") {
  await windowButton(page, t("window.size-menu")).click();
  await page.getByRole("button", { name: t(`window.${step}`), exact: true }).click();
}

/** Where an element is on the page; it must be there. */
export async function boxOf(
  element: Locator,
): Promise<{ x: number; y: number; width: number; height: number }> {
  const box = await element.boundingBox();
  if (box === null) {
    throw new TypeError("The element is not on the page.");
  }
  return box;
}

/** The text column and the column of the fields of the form, and the window, as boxes on the page. */
export async function columns(page: Page) {
  return {
    text: await boxOf(partIn(page, part.windowLayoutText)),
    fields: await boxOf(partIn(page, part.windowLayoutFields)),
    modal: await boxOf(windowBox(page)),
  };
}

/** Scrolls the form of the window to its end. */
export async function scrollFormToEnd(page: Page): Promise<void> {
  await page.evaluate((selector) => {
    const body = document.querySelector(selector);
    body?.scrollTo({ top: body.scrollHeight });
  }, `[data-part="${part.windowBody}"]`);
}

/** The size of the page as the window can fill it. */
export function pageSize(page: Page): { width: number; height: number } {
  const size = page.viewportSize();
  if (size === null) {
    throw new TypeError("The page has no size.");
  }
  return size;
}
