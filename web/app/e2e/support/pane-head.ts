import type { Locator, Page } from "@playwright/test";

import { expect } from "./fixtures.ts";
import { inOrder } from "./sequence.ts";

// The head of a pane that folds as its boxes scroll (UX §6.5): how the browser tests scroll a box as a person does and
// what they read of the head meanwhile.

/** A low window, so the boxes of the pane are higher than their room. */
export const LOW_WINDOW = { width: 1280, height: 520 } as const;

/** How far a person scrolls in one step: less than the head can fold. */
const STEP = 30;

/** The head of the pane on the screen. */
function headOf(page: Page): Locator {
  return page.locator("[data-part='pane-head']");
}

/** How high an element is drawn now. */
function heightOf(element: Locator): Promise<number> {
  return element.evaluate((node) => node.getBoundingClientRect().height);
}

/**
 * Scrolls a box as a person does, waits for the next frame, and says where the box then stands and how far its first
 * element moved on the screen.
 */
function scrollBy(box: Locator, by: number): Promise<{ scrolled: number; moved: number }> {
  return box.evaluate(async (node, step) => {
    const first = node.firstElementChild ?? node;
    const before = first.getBoundingClientRect().top;
    const start = node.scrollTop;
    node.scrollTop = start + step;
    await new Promise<void>((done) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          done();
        });
      });
    });
    return {
      scrolled: node.scrollTop - start,
      moved: before - first.getBoundingClientRect().top,
    };
  }, by);
}

/** The head folds as `box` scrolls, the box moves only by the scroll, and the head opens again at the top. */
export async function foldsWith(page: Page, box: Locator): Promise<void> {
  const head = headOf(page);
  await expect(head).toBeVisible();
  const whole = await heightOf(head);
  const step = await scrollBy(box, STEP);
  expect(step).toEqual({ scrolled: STEP, moved: STEP });
  const folding = await heightOf(head);
  expect(folding).toBeLessThan(whole);
  // Far down, the head is one line; the scroll stays where the person left it.
  await box.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  const end = await box.evaluate((node) => node.scrollTop);
  await expect.poll(() => heightOf(head)).toBeLessThan(folding);
  expect(await box.evaluate((node) => node.scrollTop)).toBe(end);
  // Back at the top the head is whole again.
  await box.evaluate((node) => {
    node.scrollTop = 0;
  });
  await expect.poll(() => heightOf(head)).toBeCloseTo(whole, 0);
  expect(await box.evaluate((node) => node.scrollTop)).toBe(0);
}

/** How many turns of the wheel a person makes, each less than the head can fold. */
const TURNS = 8;

/** Turns the wheel over `box` a few small steps down, then as many up, as a person does. */
export async function wheelOver(page: Page, box: Locator): Promise<void> {
  // Near the end of the box where the window shows it: its start is under the head, and in a low window the main
  // column may hold its end below the window.
  const spot = await box.evaluate((node) => {
    const place = node.getBoundingClientRect();
    return { x: place.left + 8, y: Math.min(place.bottom, innerHeight) - 8 };
  });
  await page.mouse.move(spot.x, spot.y);
  // One turn after the other, each once the box has taken the one before, as a hand turns the wheel.
  const turns = (way: number, from: number) =>
    Array.from({ length: TURNS }, (_, turn) => async () => {
      await page.mouse.wheel(0, way);
      await expect.poll(() => box.evaluate((node) => node.scrollTop)).toBe(from + (turn + 1) * way);
    });
  await inOrder(turns(STEP, 0));
  await inOrder(turns(-STEP, TURNS * STEP));
}
