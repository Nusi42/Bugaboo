import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { cards, moveButton, openBoard, prepareBoard, resetBoard } from "./support/board.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";

// The accessibility suite for the board on a phone (TEST-A11Y, UX §4.6): the tabs of the columns and
// the window *Move to …*, in the four themes.

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

test("a11y the board on a phone and its Move to window", async ({ page }) => {
  await signIn(page, ada);
  await openBoard(page);
  await expectGoodInEveryLook(page);
  await moveButton(page, cards.open).click();
  await expect(
    page.getByRole("dialog", { name: t("board.move-to-title", { key: cards.open }) }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});
