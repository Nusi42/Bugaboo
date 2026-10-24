import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { onThisLayout } from "./support/layout.ts";
import { startWorking } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";
import { prepareWorkflows } from "./support/workflows.ts";
import { boxOf } from "./support/windows.ts";

// Where the offer to undo stands (the final check of M4): it does not cover the Details, even when
// the Details go down to the bottom of the window.

test.use({ preparation: { run: prepareWorkflows } });

test("ux_04 the offer to undo a status change does not cover the Details", async ({ page }) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-2");
  await startWorking(page);
  // On a phone the offer stands in the column of the app, under the page; only a computer draws it over.
  await onThisLayout(page, { phone: nothing, tablet: coversNothing, computer: coversNothing });
});

const nothing = (): Promise<void> => Promise.resolve();

async function coversNothing(page: Page): Promise<void> {
  const offer = await boxOf(partIn(page, "undo-host.offer"));
  const details = await boxOf(partIn(page, "name-values"));
  const apart =
    offer.x + offer.width <= details.x ||
    details.x + details.width <= offer.x ||
    offer.y + offer.height <= details.y ||
    details.y + details.height <= offer.y;
  expect(apart).toBe(true);
}
