import type { Locator, Page } from "@playwright/test";

import { middleOf } from "./geometry.ts";
import { PANE_HEAD } from "./pane-head.ts";

// A drag made with the moves of a hand, never `dragTo`: the browser drags as it does for a person, and a drag over the
// folded head of a pane is seen by the pane (UX §6.5).

/** How many small steps a hand makes to carry a dragged thing from one place to another. */
const CARRY = 8;

/**
 * Carries what the mouse holds to the point and rests there. Chromium drops moves of a drag that come in quick
 * succession, so the last one is made again, a pixel aside and back, as a resting hand does: the browser then knows
 * where the pointer stays.
 */
async function carryTo(page: Page, to: { readonly x: number; readonly y: number }): Promise<void> {
  await page.mouse.move(to.x, to.y, { steps: CARRY });
  resting.set(page, to);
  await restAt(page, to);
}

/** Where the hand of each page rests what it holds. */
const resting = new WeakMap<Page, { readonly x: number; readonly y: number }>();

/** A pixel aside and back, as a resting hand does. */
async function restAt(page: Page, at: { readonly x: number; readonly y: number }): Promise<void> {
  await page.mouse.move(at.x + 1, at.y);
  await page.mouse.move(at.x, at.y);
}

/**
 * Holds what the mouse carries where it rests and gives what `read` reads then: a resting hand trembles, so the browser
 * keeps telling the page of the drag, as it does several times a second for a person (Playwright's mouse is still).
 */
export async function holdAndRead<Read>(page: Page, read: () => Promise<Read>): Promise<Read> {
  const at = resting.get(page);
  if (at !== undefined) {
    await restAt(page, at);
  }
  return read();
}

/**
 * Takes `handle` with the mouse and carries it over the head of the pane, where the pointer is held: the moves of a
 * hand, never `dragTo`, so the browser drags as it does for a person.
 */
export async function carryOverHead(page: Page, handle: Locator): Promise<void> {
  const from = await middleOf(handle);
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  // A drag starts once the pointer has moved a little with the button down.
  await page.mouse.move(from.x, from.y - CARRY, { steps: CARRY });
  const over = await middleOf(page.locator(PANE_HEAD));
  await carryTo(page, { x: from.x, y: over.y });
}

/** Carries what the mouse holds to the middle of `target` and lets it go there. */
export async function letGoOn(page: Page, target: Locator): Promise<void> {
  await carryTo(page, await middleOf(target));
  await page.mouse.up();
}

/** Drags the middle of `what` across by `pixels`, with the pointer. */
export async function dragAcross(page: Page, what: Locator, pixels: number): Promise<void> {
  const { x, y } = await middleOf(what);
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + pixels / 2, y, { steps: 4 });
  await page.mouse.move(x + pixels, y, { steps: 4 });
  await page.mouse.up();
}
