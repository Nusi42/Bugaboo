import { type Page } from "@playwright/test";

/** The keys of the list are for a person who is not typing: the focus leaves whatever box the page began in. */
export async function leaveBox(page: Page): Promise<void> {
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });
}
