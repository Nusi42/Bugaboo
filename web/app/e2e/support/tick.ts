import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { tickBox } from "./issues.ts";

/** How long one try to tick a box waits before the click is made again. */
const TICK_TRY_MS = 2000;

/** How long ticking one box is tried in all, far more than a computer that is busy needs. */
const TICK_TOTAL_MS = 60_000;

/**
 * Ticks the box of an issue in the list, as a person does with the label next to it. The list
 * draws only the rows in view, so the list is scrolled on until the row is there; a click that
 * lands while the rows are drawn again changes nothing, so the click is made again until the box
 * shows the tick.
 */
export async function tickIssue(page: Page, key: string): Promise<void> {
  const label = page
    .locator("label")
    .filter({ hasText: new RegExp(`^${t("issues.select-issue", { key })}$`, "u") });
  await expect(async () => {
    if ((await label.count()) === 0) {
      await page.locator("[data-part='list-scroll']").evaluate((list) => {
        list.scrollBy(0, list.clientHeight / 2);
      });
    }
    if (!(await tickBox(page, key).isChecked({ timeout: TICK_TRY_MS }))) {
      await label.click({ timeout: TICK_TRY_MS });
    }
    await expect(tickBox(page, key)).toBeChecked({ timeout: TICK_TRY_MS });
  }).toPass({ timeout: TICK_TOTAL_MS });
}
