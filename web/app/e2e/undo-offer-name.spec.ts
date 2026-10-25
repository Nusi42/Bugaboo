import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { dropFile, filesBlock, report, rowOf } from "./support/files.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { expectOfferBesideTheDetails } from "./support/undo-offer.ts";

// The offer to undo with a very long file name (the review of WP-18): the name stays whole where it
// fits, and the offer stays off the column of the Details on a computer.

test.use({ preparation: { run: prepareLook } });

/** A name far longer than a line of the offer, with hyphens to break at. */
const longName = `${"quarterly-report-of-the-whole-year-".repeat(3)}final.pdf`;

test("ux_4_4 the offer to undo a deleted file keeps the name whole and stays off the Details", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  await dropFile(
    filesBlock(page).getByRole("button", { name: t("files.drop-place"), exact: true }),
    report(longName),
  );
  await rowOf(page, longName)
    .getByRole("button", { name: t("files.delete-named", { name: longName }) })
    .click();
  const offer = partIn(page, "undo-host.offer");
  await expect(offer).toBeVisible();
  // The name is one piece of the sentence, and the sentence is not wider than the window allows.
  await expect(partIn(offer, "unbroken-word").filter({ hasText: longName })).toHaveCount(1);
  await onThisLayout(page, {
    phone: nothing,
    tablet: nothing,
    computer: expectOfferBesideTheDetails,
  });
});
