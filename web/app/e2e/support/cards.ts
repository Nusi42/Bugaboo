import { type Page } from "@playwright/test";

/** The cards of the page that show nothing: their heading is for screen readers only and nothing is in them. */
export function emptyCardsOf(page: Page): Promise<number> {
  return page.evaluate(() => {
    let empty = 0;
    for (const card of document.querySelectorAll('[data-part="card"]')) {
      const heading = card.querySelector('[data-part="card.heading"]');
      const stack = card.querySelector(':scope > [data-part="stack"]');
      if (heading === null && stack !== null && stack.children.length === 0) {
        empty += 1;
      }
    }
    return empty;
  });
}
