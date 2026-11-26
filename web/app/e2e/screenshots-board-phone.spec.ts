import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { cards, moveButton, openBoard, prepareBoard } from "./support/board.ts";
import { expect, test } from "./support/fixtures.ts";
import { inFourThemes, inFourThemesAsWindow } from "./support/four-themes.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The board on a phone in the four themes (UX §4.6): the tabs of the columns and *Move to …*. They are only
// taken when `BB_SCREENSHOT_DIR` names the folder.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareBoard } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M10/board";

test("the board and Move to …", async ({ page }) => {
  await signIn(page, ada);
  await openBoard(page);
  await inFourThemes(page, "board", MILESTONE);
  await moveButton(page, cards.open).click();
  await expect(
    page.getByRole("dialog", { name: t("board.move-to-title", { key: cards.open }) }),
  ).toBeVisible();
  await inFourThemesAsWindow(page, "move-to", MILESTONE);
  await page.keyboard.press("Escape");
});
