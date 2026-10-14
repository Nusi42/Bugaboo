import { type Locator, type Page } from "@playwright/test";

import { layoutForWidth, type LayoutMode } from "../../../packages/ui/src/layout.ts";
import { viewports } from "../../../tooling/viewports.ts";

/** Which layout the window of this page has. */
export function layoutOf(page: Page): LayoutMode {
  return layoutForWidth(page.viewportSize()?.width ?? viewports.computer.width);
}

/** What to do on a screen of each size: the test runs once per size and does what fits it. */
export type PerLayout = Readonly<Record<LayoutMode, (page: Page) => Promise<void>>>;

/** Does the step of the layout this page has. */
export function onThisLayout(page: Page, steps: PerLayout): Promise<void> {
  return steps[layoutOf(page)](page);
}

/**
 * The handle that a mouse or a finger drags a thing by. It is hidden from screen readers and left
 * out of the tab order (the buttons next to it do the same things), so it is found by its part and
 * not by its role.
 */
export function handleIn(scope: Locator | Page, label: string): Locator {
  return scope.locator(`[data-part="draggable.handle"][aria-label="${label}"]`);
}

/** The places where a dragged thing can be dropped. */
export function slotsIn(scope: Locator): Locator {
  return scope.locator('[data-part="drop-slot"]');
}
