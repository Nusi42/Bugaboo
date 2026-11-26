import { t } from "@bugaboo/messages";

import { ada, signInThroughApi } from "./support/accounts.ts";
import {
  cardOf,
  cards,
  columnOf,
  openBoard,
  prepareBoard,
  resetBoard,
  statusNames,
  statusOfIssue,
} from "./support/board.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { letGoOn, takeUp } from "./support/hand-drag.ts";
import { partIn } from "./support/parts.ts";
import { askForWhoWorks } from "./support/workflows.ts";

// A drop that asks for a value (PLAN-06, UX §4.6): the window of the issue page opens, and cancelling it leaves the card.

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

/** The grip of a card: its first line, which is no link. */
function grip(page: Parameters<typeof cardOf>[0], key: string) {
  return partIn(cardOf(page, key), "board-card.line");
}

test("plan_06_a_drop_that_asks_for_a_value_opens_the_window_of_the_issue_page", async ({
  page,
  server,
}) => {
  const admin = await signInThroughApi(server, ada);
  await askForWhoWorks(admin, "WEB");
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.open;
  await takeUp(page, grip(page, key));
  await letGoOn(page, columnOf(page, statusNames.progress));
  const window = page.getByRole("dialog", { name: /Start working/u });
  await expect(window).toBeVisible();
  // Cancelling leaves the card where it was.
  await window.getByRole("button", { name: t("common.cancel") }).click();
  await expect(window).toBeHidden();
  await expect(columnOf(page, statusNames.open).getByText(key)).toBeVisible();
  expect(await statusOfIssue(admin, key)).toBe(statusNames.open);
  await admin.dispose();
});
