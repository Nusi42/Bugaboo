import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { layoutForWidth, type LayoutMode } from "../../../packages/ui/src/layout.ts";
import { viewports } from "../../../tooling/viewports.ts";

export { nothing } from "./sequence.ts";

/** Which layout the window of this page has. */
export function layoutOf(page: Page): LayoutMode {
  return layoutForWidth(page.viewportSize()?.width ?? viewports.computer.width);
}

/** *Back* from an open issue: *Back to list* of the focus view, or *Back* of the top bar on a phone (UX §4.4, L1). */
export function issueBackOf(page: Page): Locator {
  return layoutOf(page) === "phone"
    ? page.getByRole("banner").getByRole("button", { name: t("shell.back"), exact: true })
    : page.getByRole("button", { name: t("focus.back"), exact: true });
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

/** The elements stand in one column and are as wide as each other (the choices of a list on a phone). */
export async function expectOneColumn(elements: readonly Locator[]): Promise<void> {
  const boxes = await Promise.all(elements.map((element) => element.boundingBox()));
  const first = boxes[0];
  if (first === null || first === undefined) {
    throw new TypeError("The first element has no box.");
  }
  for (const box of boxes) {
    expect(Math.abs((box?.x ?? Number.NaN) - first.x)).toBeLessThanOrEqual(2);
    expect(Math.abs((box?.width ?? Number.NaN) - first.width)).toBeLessThanOrEqual(2);
  }
}

/** The place where one item of a list is changed: the panel beside the list on a computer, its own screen on a phone. */
export function itemArea(page: Page, name: string): Locator {
  return layoutOf(page) === "phone"
    ? page.getByRole("region", { name, exact: true })
    : page.getByRole("complementary", { name });
}
