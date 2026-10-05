import { type Page } from "@playwright/test";

/** The three layouts of the app (UX §3) and the width at which each starts. */
export type Layout = "computer" | "tablet" | "phone";
const COMPUTER_FROM = 1280;
const TABLET_FROM = 768;

/** Which layout the window of this page has. */
export function layoutOf(page: Page): Layout {
  const width = page.viewportSize()?.width ?? COMPUTER_FROM;
  if (width >= COMPUTER_FROM) {
    return "computer";
  }
  return width >= TABLET_FROM ? "tablet" : "phone";
}

/** What to do on a screen of each size: the test runs once per size and does what fits it. */
export type PerLayout = Readonly<Record<Layout, (page: Page) => Promise<void>>>;

/** Does the step of the layout this page has. */
export function onThisLayout(page: Page, steps: PerLayout): Promise<void> {
  return steps[layoutOf(page)](page);
}
