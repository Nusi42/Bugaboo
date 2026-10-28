import { type Page } from "@playwright/test";

/**
 * The buttons of the page that overlap the button before them in the same row (a neighbour in the
 * same parent on the same line), by their text. A quiet button hangs back by its padding; where the
 * edge of a button is drawn (forced colors) it must not touch the one before it.
 */
export function buttonsThatTouch(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const found: string[] = [];
    const last = new Map<Element, DOMRect>();
    for (const button of document.querySelectorAll('[data-part="button"]')) {
      const parent = button.parentElement;
      const box = button.getBoundingClientRect();
      const before = parent === null ? undefined : last.get(parent);
      const sameLine = before !== undefined && before.top < box.bottom && box.top < before.bottom;
      if (before !== undefined && sameLine && box.left < before.right - 0.5) {
        found.push(button.textContent);
      }
      if (parent !== null) {
        last.set(parent, box);
      }
    }
    return found;
  });
}
