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

/** How far the content of the first card starts under its top edge, and the padding of the card: no slot of a hidden heading is between. */
export function slotAboveContent(page: Page): Promise<{ gap: number; padding: number }> {
  return page.evaluate(() => {
    const card = document.querySelector('[data-part="card"]');
    const stack = card?.querySelector(':scope > [data-part="stack"]');
    if (card === null || card === undefined || stack === null || stack === undefined) {
      return { gap: 0, padding: 0 };
    }
    return {
      gap: stack.getBoundingClientRect().top - card.getBoundingClientRect().top,
      padding: Number(getComputedStyle(card).paddingTop.replace("px", "")),
    };
  });
}
