import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { cardOf, cards, actionsOf, openBoard, prepareBoard } from "./support/board.ts";
import { resetBoard } from "./support/board-reset.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";

// The accessibility suite for the board on a computer (TEST-A11Y, UX §4.6): the columns and cards,
// the window *Move to …*, a card picked up by its keys, and *Customize*, in the four themes. Every
// drag of the board has its key; the tests of `board-computer.spec.ts` use them.

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

test("a11y the board, a card in hand and its windows, with the keyboard only", async ({ page }) => {
  await signIn(page, ada);
  await openBoard(page);
  await expectGoodInEveryLook(page);
  const key = cards.open;
  await actionsOf(page, key).focus();
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: t("board.move-to"), exact: true }).focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", { name: t("board.move-to-title", { key }) });
  await expect(window).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
  // The card is picked up by Space: the columns it cannot go to say why.
  await cardOf(page, key).getByRole("link").focus();
  await page.keyboard.press("Space");
  await expect(cardOf(page, key)).toHaveAttribute("data-state", /picked/u);
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(cardOf(page, key)).not.toHaveAttribute("data-state", /picked/u);
  await page.getByRole("button", { name: t("board.customize"), exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog", { name: t("board.customize-title") })).toBeVisible();
  await expectGoodInEveryLook(page);
});
