import { expect, type Page } from "@playwright/test";

import { nothing, onThisLayout } from "./layout.ts";
import { menuEntry } from "./notifications.ts";

/**
 * In the narrow menu the number stands under the bell, whatever its width ("100+" is wider than the
 * bell, so a place beside it would not be clear); it never covers any of it.
 */
export function expectCountUnderTheBell(page: Page): Promise<void> {
  return onThisLayout(page, {
    // The menu a tablet slides in is wide, and the bar of a phone has its number after the bell.
    phone: nothing,
    tablet: nothing,
    computer: async () => {
      const entry = menuEntry(page);
      // How far the number stands under the bell, read in one go so a menu that is drawn again between two
      // readings cannot make them disagree; it is not a number while one of them is not drawn yet.
      await expect
        .poll(() =>
          entry.evaluate((node) => {
            const count = node.querySelector('[data-part="nav-item.count"]');
            const bell = node.querySelector("svg");
            return count === null || bell === null
              ? Number.NaN
              : count.getBoundingClientRect().top - bell.getBoundingClientRect().bottom;
          }),
        )
        .toBeGreaterThanOrEqual(0);
    },
  });
}
