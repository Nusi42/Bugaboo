import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { themes, type Theme } from "./checks.ts";
import { boxOf, shadowsOf } from "./windows.ts";

// The checks of the issue page on a phone (UX §4.4, M5 exit L1): its bar of actions, its colors and its shadow.

/** Daylight, whose page is white like a bar that has no surface of its own. */
export function daylight(): Theme {
  const found = themes.find((theme) => theme.id === "daylight");
  if (found === undefined) {
    throw new TypeError("Daylight is a built-in theme.");
  }
  return found;
}

/** The top edges of the buttons with these words. */
export function topsOf(
  page: Page,
  keys: readonly ("list.previous" | "list.next")[],
): Promise<number[]> {
  return Promise.all(
    keys.map(
      async (key) => (await boxOf(page.getByRole("button", { name: t(key), exact: true }))).y,
    ),
  );
}

/** How far the most and the least of the numbers are apart. */
export function spread(numbers: readonly number[]): number {
  return Math.max(...numbers) - Math.min(...numbers);
}

/** The bar of actions fills its width in equal parts: the same width each, the same room at both ends and between them. */
export async function expectEqualParts(bar: Locator): Promise<void> {
  const buttons = bar.getByRole("button");
  await expect(buttons).toHaveCount(3);
  const boxes = await Promise.all([0, 1, 2].map((index) => boxOf(buttons.nth(index))));
  const whole = await boxOf(bar);
  const first = boxes.at(0);
  const last = boxes.at(-1);
  if (first === undefined || last === undefined) {
    throw new TypeError("The bar has three buttons.");
  }
  const widths = await Promise.all(
    [0, 1, 2].map(async (index) => (await boxOf(buttons.nth(index))).width),
  );
  expect(spread(widths)).toBeLessThanOrEqual(1);
  const before = first.x - whole.x;
  const after = whole.x + whole.width - (last.x + last.width);
  expect(Math.abs(before - after)).toBeLessThanOrEqual(1);
  // The gaps between the buttons are the padding of the bar: the three fill it.
  expect(first.width * 3 + before * 4).toBeGreaterThanOrEqual(whole.width - 2);
}

/** The color behind a part. */
export function backgroundOf(part: Locator): Promise<string> {
  return part.evaluate((node) => getComputedStyle(node).backgroundColor);
}

/** Whether the shadow at the end of a scrolling box is there (1) or not (0). */
export async function shadowAtEndOf(box: Locator): Promise<number> {
  return (await shadowsOf(box)).end;
}

/** The element is as wide as the box it stands in (a pixel of rounding). */
export async function expectFullWidth(element: Locator, box: Locator): Promise<void> {
  expect((await boxOf(element)).width).toBeGreaterThanOrEqual((await boxOf(box)).width - 1);
}
