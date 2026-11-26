import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  cardOf,
  chainPanel,
  mapList,
  mapped,
  openMap,
  prepareRelationMap,
} from "./support/relation-map.ts";

// The accessibility suite of the relation map on a computer (TEST-A11Y, UX §4.15): the picture with
// a chain chosen by the keyboard, and the list form, in the four themes.

test.use({ preparation: { run: prepareRelationMap } });

test("a11y the picture of the relation map and the chain of a card, with the keyboard only", async ({
  page,
}) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await expectGoodInEveryLook(page);
  await cardOf(page, mapped.timesheet).focus();
  await page.keyboard.press("Enter");
  await expect(cardOf(page, mapped.timesheet)).toHaveAttribute("aria-pressed", "true");
  await expect(chainPanel(page, mapped.timesheet)).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the list form of the relation map", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, mapped.start);
  await page.getByRole("radio", { name: t("relation-map.list"), exact: true }).check();
  await expect(mapList(page)).toBeVisible();
  await expectGoodInEveryLook(page);
});
