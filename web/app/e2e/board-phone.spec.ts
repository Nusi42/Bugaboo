import { t } from "@bugaboo/messages";

import { ada, signInThroughApi } from "./support/accounts.ts";
import {
  cardOf,
  cards,
  openMoveTo,
  openBoard,
  prepareBoard,
  statusNames,
  statusOfIssue,
} from "./support/board.ts";
import { resetBoard } from "./support/board-reset.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";

// The board on a phone (PLAN-06, UX §4.6): one column at a time, the names of the columns as tabs with
// their counts, and every card moved with *Move to …*: nothing is dragged.

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

test("plan_06_the_phone_shows_one_column_with_tabs_and_moves_by_move_to", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openBoard(page);
  const tabs = page.getByRole("tablist", { name: t("board.tabs") });
  const named = (name: string, count: number) =>
    tabs.getByRole("tab", { name: t("board.tab", { name, count }), exact: true });
  await expect(named(statusNames.open, 3)).toBeVisible();
  await expect(named(statusNames.progress, 3)).toBeVisible();
  await expect(named(statusNames.done, 1)).toBeVisible();
  // One column at a time.
  await expect(cardOf(page, cards.working)).toBeHidden();
  await named(statusNames.progress, 3).click();
  await expect(cardOf(page, cards.working)).toBeVisible();
  await expect(cardOf(page, cards.open)).toBeHidden();
  // Nothing is dragged; *Move to …* is the way.
  await expect(cardOf(page, cards.working)).not.toHaveAttribute("draggable", "true");
  const window = await openMoveTo(page, cards.working);
  await window.getByRole("button", { name: statusNames.done, exact: true }).click();
  await expect(cardOf(page, cards.working)).toBeHidden();
  const admin = await signInThroughApi(server, ada);
  expect(await statusOfIssue(admin, cards.working)).toBe(statusNames.done);
  await admin.dispose();
});
