import { type Page } from "@playwright/test";

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
