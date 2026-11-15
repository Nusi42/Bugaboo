import { type Locator, type Page } from "@playwright/test";

import { boxOf } from "./geometry.ts";

/** The parts of the page that scroll now (their own scroll bar), by their parts: a phone list has one. */
export function scrollersOf(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const found: string[] = [];
    for (const element of document.querySelectorAll("*")) {
      const style = getComputedStyle(element);
      const scrolls = style.overflowY === "auto" || style.overflowY === "scroll";
      if (scrolls && element.scrollHeight > element.clientHeight + 1) {
        found.push(
          element instanceof HTMLElement
            ? (element.dataset["part"] ?? element.tagName)
            : element.tagName,
        );
      }
    }
    return found;
  });
}

/**
 * The place, among the cards, of the last one that is whole in sight: inside the box of the list
 * and above `end` (the top of the bar that stands over its foot); -1 if none is.
 */
export async function lastWholeCard(cards: Locator, list: Locator, end: number): Promise<number> {
  const box = await boxOf(list);
  const count = await cards.count();
  const boxes = await Promise.all(Array.from({ length: count }, (_, at) => boxOf(cards.nth(at))));
  let found = -1;
  for (const [at, card] of boxes.entries()) {
    if (card.y + card.height <= Math.min(box.y + box.height, end) + 1) {
      found = at;
    }
  }
  return found;
}

/** The texts of the cells that do not show all of their words (cut at the edge, with dots or without); none when every word shows. */
export function cutWords(cells: Locator): Promise<(string | null)[]> {
  return cells.evaluateAll((all: readonly HTMLElement[]) => {
    const cut: (string | null)[] = [];
    for (const cell of all) {
      if (cell.scrollWidth > cell.clientWidth + 1) {
        cut.push(cell.textContent);
      }
    }
    return cut;
  });
}

/** How many lines each element takes, from the boxes of its lines (one box for each line it is drawn on). */
export function linesOfEach(elements: Locator): Promise<number[]> {
  return elements.evaluateAll((all: readonly HTMLElement[]) => {
    const lines: number[] = [];
    for (const element of all) {
      const tops = new Set<number>();
      for (const box of element.getClientRects()) {
        tops.add(Math.round(box.top));
      }
      lines.push(tops.size);
    }
    return lines;
  });
}
