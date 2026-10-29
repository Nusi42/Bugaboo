import { expect } from "@playwright/test";

import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { paths } from "../src/shared/paths.ts";
import type { Page } from "@playwright/test";

// M5 exit, package O: a mention has no room of its own at its ends, so the comma after it stands against it.

test.use({ sample: true });

/** How far the first character after the first mention is from the mention, where a comma follows it. */
function gapAfterMention(page: Page): Promise<number> {
  return page.evaluate(() => {
    for (const token of document.querySelectorAll("[data-token][data-kind='mention']")) {
      const next = token.nextSibling;
      if (next?.nodeType === Node.TEXT_NODE && next.textContent?.startsWith(",") === true) {
        const range = document.createRange();
        range.setStart(next, 0);
        range.setEnd(next, 1);
        return range.getBoundingClientRect().left - token.getBoundingClientRect().right;
      }
    }
    return Number.NaN;
  });
}

test("ux_8 a comma after a mention stands against it", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.home);
  await expect(page.locator("[data-token][data-kind='mention']").first()).toBeVisible();
  const gap = await gapAfterMention(page);
  expect(gap, "the room between a mention and the comma after it").toBeLessThanOrEqual(1);
});
