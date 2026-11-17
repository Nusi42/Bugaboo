import { expect } from "@playwright/test";
import { type Page } from "@playwright/test";

import { storageKey } from "../../../packages/ui/src/storage.ts";
import { issueAddress, paths } from "../../src/shared/paths.ts";

// The menu never scrolls sideways (UX §6.5): none of its boxes holds more across than it shows.

/** The menu wide, narrowed by the person (what the device remembers), and narrowed by a screen with a list. */
export type MenuState = "wide" | "narrowed" | "squeezed";

/** The widest a narrowed menu is, in pixels: the wide one is more than twice that. */
const NARROW_AT_MOST = 100;

/** Opens a screen with the menu in `state`. */
export async function openWithMenu(page: Page, state: MenuState): Promise<void> {
  if (state === "narrowed") {
    // As the person left it; in a low window the button for it is folded away in *Settings and more*.
    await page.addInitScript((key) => {
      localStorage.setItem(key, "yes");
    }, storageKey.sidebarNarrow);
  }
  await page.goto(state === "squeezed" ? issueAddress("WEB-1") : paths.home);
  const menu = page.locator("[data-part='sidebar']");
  await expect(menu).toBeVisible();
  const width = await menu.evaluate((node) => node.getBoundingClientRect().width);
  expect(width < NARROW_AT_MOST, "the menu is narrowed").toBe(state !== "wide");
}

/** Expects that no box of the menu that clips or scrolls holds more across than it shows. */
export async function expectMenuNeverScrollsSideways(page: Page): Promise<void> {
  const boxes = await page.evaluate(() => {
    const found: string[] = [];
    // The words kept for screen readers are a clipped box of one pixel by design.
    for (const box of document.querySelectorAll(
      "[data-part='sidebar'], [data-part='sidebar'] :not([data-part='visually-hidden'])",
    )) {
      if (getComputedStyle(box).overflowX !== "visible" && box.scrollWidth > box.clientWidth) {
        found.push(
          `${box instanceof HTMLElement ? (box.dataset["part"] ?? box.tagName) : box.tagName}: ${box.scrollWidth - box.clientWidth}px more`,
        );
      }
    }
    return found;
  });
  expect(boxes, "the boxes of the menu that hold more across than they show").toEqual([]);
}
