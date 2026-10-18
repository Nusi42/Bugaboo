import { expect, type Page } from "@playwright/test";

import { theTwo } from "./demo-m3-state.ts";
import { good, openTheBug } from "./demo-m4-page.ts";
import { writeTheCustomer } from "./demo-m4-prepare.ts";
import { CUSTOMER, PASTED } from "./demo-m4-state.ts";
import { PREVIEW_WAIT_MS, pasteFile, previewOf, rowOf, screenshot } from "./files.ts";
import { signIn } from "./flows.ts";
import { type Instance } from "./server.ts";

/** Step 3: the Team member pastes a screenshot onto the bug, and writes the customer. */
export async function aScreenshotIsPasted(page: Page, server: Instance): Promise<void> {
  await signIn(page, theTwo().first);
  await openTheBug(page);
  await pasteFile(page, screenshot(PASTED));
  await expect(rowOf(page, PASTED)).toBeVisible();
  await expect(previewOf(page, PASTED)).toBeVisible({ timeout: PREVIEW_WAIT_MS });
  await writeTheCustomer(server);
  await page.reload();
  await expect(page.getByText(CUSTOMER)).toBeVisible();
  await good(page, "04-screenshot-pasted");
}
