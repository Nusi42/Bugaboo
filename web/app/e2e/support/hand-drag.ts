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
  await page.mouse.move(to.x + 1, to.y);
  await page.mouse.move(to.x, to.y);
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
