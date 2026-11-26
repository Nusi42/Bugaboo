import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import {
  boardOn,
  cardOf,
  cards,
  columnOf,
  moveButton,
  openBoard,
  prepareBoard,
  statusNames,
} from "./support/board.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { middleOf } from "./support/geometry.ts";
import { takeUp } from "./support/hand-drag.ts";
import { partIn } from "./support/parts.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The board in the four themes, on a computer (UX §4.6): the columns with a column over
// its card limit, a card in hand with the column it cannot go to faded, *Move to …*, and *Customize*. They
// are only taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareBoard } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M10/board";

test("the board, a card in hand, Move to … and Customize", async ({ page }) => {
  await signIn(page, ada);
  await openBoard(page);
  await inFourThemes(page, "board", MILESTONE);
  await moveButton(page, cards.open).click();
  await expect(
    page.getByRole("dialog", { name: t("board.move-to-title", { key: cards.open }) }),
  ).toBeVisible();
  await inFourThemesAsWindow(page, "move-to", MILESTONE);
  await page.keyboard.press("Escape");
  await takeUp(page, partIn(cardOf(page, cards.bug), "board-card.line"));
  const over = await middleOf(columnOf(page, statusNames.done));
  await page.mouse.move(over.x, over.y, { steps: 6 });
  await expect(partIn(boardOn(page), "board-column.reason").first()).toBeVisible();
  await inFourThemes(page, "card-in-hand", MILESTONE);
  await page.mouse.up();
  await page.getByRole("button", { name: t("board.customize"), exact: true }).click();
  await inFourThemesAsWindow(page, "customize", MILESTONE);
});
