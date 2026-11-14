import { type Page } from "@playwright/test";

// What scrolls on a screen (UX §6.5): the page itself, and the boxes that scroll up and down.

/** How far the page itself scrolls, and the boxes that scroll up and down inside another such box, by their parts. */
export interface Scrolling {
  readonly page: number;
  readonly nested: readonly string[];
}

/** What scrolls on the screen now. */
export function scrollingOf(page: Page): Promise<Scrolling> {
  return page.evaluate(() => {
    const frame = document.scrollingElement ?? document.documentElement;
    const boxes: Element[] = [];
    for (const element of document.querySelectorAll("*")) {
      const { overflowY } = getComputedStyle(element);
      const scrolls = overflowY === "auto" || overflowY === "scroll";
      if (scrolls && element.scrollHeight > element.clientHeight + 1) {
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
