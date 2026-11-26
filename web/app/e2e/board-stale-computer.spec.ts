import { ada, bob, signInThroughApi } from "./support/accounts.ts";
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
import { conflictNote, keepTheirs } from "./support/saved.ts";
import { changeIssue } from "./support/timeline.ts";

// A move that somebody else's change came before (PLAN-06, UX §4.6): *Keep Ben's change* or *Use mine*.

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

/** The grip of a card: its first line, which is no link. */
function grip(page: Parameters<typeof cardOf>[0], key: string) {
  return partIn(cardOf(page, key), "board-card.line");
}

test("plan_06_a_stale_move_asks_keep_bens_change_or_use_mine", async ({ page, server }) => {
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.open;
  // Ben changes the issue after the page read it.
  const ben = await signInThroughApi(server, bob);
  await changeIssue(ben, key, { priority: "high" });
  await takeUp(page, grip(page, key));
  await letGoOn(page, columnOf(page, statusNames.progress));
  const mine = conflictNote(page);
  await expect(mine).toBeVisible();
  await expect(keepTheirs(page, bob.name)).toBeVisible();
  await mine.click();
  await expect(columnOf(page, statusNames.progress).getByText(key)).toBeVisible();
  const admin = await signInThroughApi(server, ada);
  expect(await statusOfIssue(admin, key)).toBe(statusNames.progress);
  await admin.dispose();
  await ben.dispose();
});
