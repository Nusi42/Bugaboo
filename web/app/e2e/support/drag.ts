import { type Locator } from "@playwright/test";

/**
 * The handle that a mouse or a finger drags a thing by. It is hidden from screen readers and left
 * out of the tab order (the buttons next to it do the same things), so it is found by its part and
 * not by its role.
 */
export function handleIn(scope: Locator, label: string): Locator {
  return scope.locator(`[data-part="draggable.handle"][aria-label="${label}"]`);
}

/** The places where a dragged thing can be dropped. */
export function slotsIn(scope: Locator): Locator {
  return scope.locator('[data-part="drop-slot"]');
}
