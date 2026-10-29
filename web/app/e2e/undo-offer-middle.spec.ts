import { t } from "@bugaboo/messages";

import { ada, bob, createAdmin, createPerson } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { itemArea, nothing, onThisLayout } from "./support/layout.ts";
import {
  expectOfferClearOfControls,
  expectOfferInThePageColumn,
  expectOfferInTheMiddleOfThePage,
  inEveryComputerWidth,
} from "./support/undo-offer.ts";

// Where the offer to undo stands on a page without Details (the re-check of WP-18): in the middle of
// the column the page is drawn in, in a window of any width, with the sidebar wide or narrowed, and a
// long offer keeps off the controls of the side panel (UX §6, *Where the offer to undo stands*).

/** A long name, so that the offer saying who joined it is long too. */
const groupName = "Backend, platform and operations team of the northern region";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await (await createPerson(server, admin, bob)).dispose();
      await admin.dispose();
    },
  },
});

test("ux_04 the offer to undo on a page without Details stands in the middle, with the sidebar wide or narrow", async ({
  page,
}) => {
  await onThisLayout(page, {
    phone: nothing,
    tablet: nothing,
    computer: async () => {
      await signIn(page, ada);
      await openFromMenu(page, t("nav.groups"));
      await page.getByLabel(t("groups.field-name"), { exact: true }).fill(groupName);
      await page.getByRole("button", { name: t("groups.create"), exact: true }).click();
      await page.getByRole("button", { name: groupName, exact: true }).click();
      const panel = itemArea(page, groupName);
      await panel.getByRole("searchbox", { name: t("groups.add-search") }).fill("builder");
      await panel.getByRole("button", { name: t("groups.add-name", { name: bob.name }) }).click();
      await expect(
        page.getByText(t("groups.added", { name: bob.name, group: groupName })),
      ).toBeVisible();
      await inEveryComputerWidth(page, async () => {
        await expectOfferInThePageColumn(page);
        await expectOfferInTheMiddleOfThePage(page);
        await expectOfferClearOfControls(page, panel);
      });
      await page.getByRole("button", { name: t("shell.narrow"), exact: true }).click();
      await expectOfferInTheMiddleOfThePage(page);
    },
  });
});
