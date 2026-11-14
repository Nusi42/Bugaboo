import { type Page } from "@playwright/test";

// What scrolls on a screen (UX §6.5): the page itself, and the boxes that scroll up and down.

/**
 * How far the page itself scrolls, and the boxes that scroll up and down inside another such box, by their parts. A
 * box counts by its look, whatever it holds now: one that has little in it on one screen scrolls inside the other once
 * it holds more.
 */
export interface Scrolling {
  readonly page: number;
  readonly nested: readonly string[];
}

/** A window as low as a phone held sideways, or a phone with the browser's bars: the pages before sign-in still fit it. */
const LOW_WINDOW_HEIGHT = 360;

/** Makes the window low, keeping its width. */
export async function lowWindow(page: Page): Promise<void> {
  const width = page.viewportSize()?.width ?? 0;
  await page.setViewportSize({ width, height: LOW_WINDOW_HEIGHT });
}

/** What scrolls on the screen now. */
export function scrollingOf(page: Page): Promise<Scrolling> {
  return page.evaluate(() => {
    const frame = document.scrollingElement ?? document.documentElement;
    const boxes: Element[] = [];
    // A box to type in scrolls the words typed in it: a control, not a box of the page.
    for (const element of document.querySelectorAll(":not(textarea, input, select)")) {
      const { overflowY } = getComputedStyle(element);
      if (overflowY === "auto" || overflowY === "scroll") {
        boxes.push(element);
      }
    }
    const nested: string[] = [];
    for (const box of boxes) {
      let inside = false;
      for (const other of boxes) {
        inside ||= other !== box && other.contains(box);
      }
      if (inside) {
        nested.push(
          box instanceof HTMLElement ? (box.dataset["part"] ?? box.tagName) : box.tagName,
        );
      }
    }
    return { page: Math.max(0, frame.scrollHeight - frame.clientHeight), nested };
  });
}
