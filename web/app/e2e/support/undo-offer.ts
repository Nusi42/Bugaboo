import { expect, type Locator, type Page } from "@playwright/test";

import { partIn } from "./parts.ts";
import { inOrder } from "./sequence.ts";
import { boxOf } from "./windows.ts";

type Box = Readonly<Awaited<ReturnType<typeof boxOf>>>;

/** The two boxes lie apart: one ends before the other starts, across or down. */
function lieApart(a: Box, b: Box): boolean {
  return (
    a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y
  );
}

/** The offer to undo does not cover the Details: the two boxes lie apart. */
export async function expectOfferApartFromDetails(page: Page): Promise<void> {
  const offer = await boxOf(partIn(page, "undo-host.offer"));
  const details = await boxOf(partIn(page, "name-values"));
  expect(lieApart(offer, details)).toBe(true);
}

/** The offer to undo covers no button or box of the panel: each control of it lies apart from the offer. */
export async function expectOfferClearOfControls(page: Page, panel: Locator): Promise<void> {
  const offer = await boxOf(partIn(page, "undo-host.offer"));
  const controls = await panel.locator("button, input, select, textarea").all();
  expect(controls.length).toBeGreaterThan(0);
  const shown = await Promise.all(
    controls.map(async (control) => ((await control.isVisible()) ? boxOf(control) : null)),
  );
  for (const box of shown) {
    if (box !== null) {
      expect(lieApart(offer, box)).toBe(true);
    }
  }
}

/**
 * The offer to undo ends before the column of the Details starts, wherever the page is scrolled to:
 * a long message wraps inside that room and never reaches over the Details.
 */
export async function expectOfferBesideTheDetails(page: Page): Promise<void> {
  const offer = await boxOf(partIn(page, "undo-host.offer"));
  const details = await boxOf(partIn(page, "name-values"));
  expect(offer.x + offer.width).toBeLessThanOrEqual(details.x);
}

/** The offer to undo lies inside the column the page is drawn in, whatever the window and the sidebar are. */
export async function expectOfferInThePageColumn(page: Page): Promise<void> {
  const offer = await boxOf(partIn(page, "undo-host.offer"));
  const column = await boxOf(partIn(page, "app-layout.inner"));
  expect(offer.x).toBeGreaterThanOrEqual(column.x - 1);
  expect(offer.x + offer.width).toBeLessThanOrEqual(column.x + column.width + 1);
}

/** The offer to undo stands in the middle of the page column: a page without Details has nothing to keep clear. */
export async function expectOfferInTheMiddleOfThePage(page: Page): Promise<void> {
  const offer = await boxOf(partIn(page, "undo-host.offer"));
  const column = await boxOf(partIn(page, "app-layout.inner"));
  expect(Math.abs(offer.x + offer.width / 2 - (column.x + column.width / 2))).toBeLessThanOrEqual(
    2,
  );
}

/** Windows of a computer: the narrowest, a wide one and a very wide one. */
const computerWidths = [1280, 1440, 1920] as const;
const WINDOW_HEIGHT = 900;

/** Looks in each of the windows of a computer, one after the other, at what the check looks at. */
export function inEveryComputerWidth(
  page: Page,
  check: (page: Page) => Promise<void>,
): Promise<void> {
  return inOrder(
    computerWidths.map((width) => async () => {
      await page.setViewportSize({ width, height: WINDOW_HEIGHT });
      await check(page);
    }),
  );
}
