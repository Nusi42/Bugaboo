import { ada } from "./support/accounts.ts";
import { test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { startWorking } from "./support/issues.ts";
import { prepareWorkflows } from "./support/workflows.ts";
import { expectOfferApartFromDetails } from "./support/undo-offer.ts";

// Where the offer to undo stands (the final check of M4): it does not cover the Details, even when
// the Details go down to the bottom of the window.

test.use({ preparation: { run: prepareWorkflows } });

test("ux_04 the offer to undo a status change does not cover the Details", async ({ page }) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-2");
  await startWorking(page);
  // On a phone the offer stands in the column of the app, under the page; only a computer draws it over.
  await onThisLayout(page, {
    phone: nothing,
    tablet: expectOfferApartFromDetails,
    computer: expectOfferApartFromDetails,
  });
});
