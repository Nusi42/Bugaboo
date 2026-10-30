import { type Locator, type Page } from "@playwright/test";

import { boxOf } from "./windows.ts";

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

/** Scrolls an element to its end, until nothing more is loaded under it. */
export function toTheEnd(box: Locator): Promise<number> {
  return box.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
    return element.scrollHeight - element.clientHeight - element.scrollTop;
  });
}

/** The place, among the cards, of the last one that is whole inside the box of the list; -1 if none is. */
export async function lastWholeCard(cards: Locator, list: Locator): Promise<number> {
  const box = await boxOf(list);
  const count = await cards.count();
  const boxes = await Promise.all(Array.from({ length: count }, (_, at) => boxOf(cards.nth(at))));
  let found = -1;
  for (const [at, card] of boxes.entries()) {
    if (card.y + card.height <= box.y + box.height + 1) {
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
