import { expect, type Page } from "@playwright/test";

import { nothing, onThisLayout } from "./layout.ts";
import { menuEntry } from "./notifications.ts";
import { partIn } from "./parts.ts";
import { boxOf } from "./windows.ts";

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
      const count = await boxOf(partIn(entry, "nav-item.count"));
      const bell = await boxOf(entry.locator("svg").first());
      expect(count.y).toBeGreaterThanOrEqual(bell.y + bell.height);
    },
  });
}
