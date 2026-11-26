import { ada } from "./support/accounts.ts";
import { cards, openMoveTo, openBoard, prepareBoard } from "./support/board.ts";
import { resetBoard } from "./support/board-reset.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { test } from "./support/fixtures.ts";
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
  await openMoveTo(page, cards.open);
  await expectGoodInEveryLook(page);
});
