import { t } from "@bugaboo/messages";

import { ada, bob, idOf, signInThroughApi } from "./support/accounts.ts";
import {
  boardName,
  boardOn,
  cardOf,
  cards,
  columnOf,
  headOf,
  moveButton,
  openBoard,
  prepareBoard,
  resetBoard,
  progressLimit,
  saveBoard,
  savedIdOf,
  shareWithWeb,
  statusNames,
  statusOfIssue,
} from "./support/board.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { boxOf, middleOf } from "./support/geometry.ts";
import { letGoOn, takeUp } from "./support/hand-drag.ts";
import { assignTo, issueOf, versionOf } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";
import { savedSearchAddress } from "../src/shared/paths.ts";

// The board on a computer (PLAN-06, UX §4.6): a saved search shown as columns of cards, a drag that
// changes the status through the workflow, the columns a card cannot go to, the card limit, and
// *Move to …* with the keyboard only.

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

/** The part of a card a hand takes: its first line, which is no link. */
function grip(page: Parameters<typeof cardOf>[0], key: string) {
  return partIn(cardOf(page, key), "board-card.line");
}

test("plan_06_a_saved_search_shows_as_a_board_with_columns_rows_and_counts", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openBoard(page);
  await expect(headOf(page, statusNames.open)).toContainText(t("board.count", { count: 3 }));
  await expect(headOf(page, statusNames.done)).toContainText(t("board.count", { count: 1 }));
  await expect(
    columnOf(page, statusNames.open).getByRole("link", { name: /Login fails/u }),
  ).toBeVisible();
  // Rows: by the person the issues are assigned to.
  const admin = await signInThroughApi(server, ada);
  const issue = await issueOf(admin, cards.working);
  await assignTo(admin, issue, await idOf(server, bob), await versionOf(admin, issue));
  const rowsId = await saveBoard(admin, "Rows board", {
    columns: null,
    rows: "assigned to",
    card_limits: [],
  });
  await page.goto(savedSearchAddress(rowsId));
  const rows = partIn(page, "board-row");
  await expect(rows).toHaveCount(2);
  await expect(partIn(rows.first(), "board-row.toggle")).toHaveAttribute("aria-expanded", "true");
  await admin.dispose();
});

test("plan_06_dragging_a_card_changes_its_status_and_undo_takes_it_back", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openBoard(page);
  const head = await boxOf(headOf(page, statusNames.done));
  const untouched = await boxOf(cardOf(page, cards.done));
  const key = cards.open;
  await takeUp(page, grip(page, key));
  await letGoOn(page, columnOf(page, statusNames.progress));
  const moved = t("board.moved", { key, column: statusNames.progress });
  await expect(page.getByText(moved)).toBeVisible();
  await expect(columnOf(page, statusNames.progress).getByText(key)).toBeVisible();
  const admin = await signInThroughApi(server, ada);
  expect(await statusOfIssue(admin, key)).toBe(statusNames.progress);
  // The columns stay where they were: only the card moved.
  expect(await boxOf(headOf(page, statusNames.done))).toEqual(head);
  expect(await boxOf(cardOf(page, cards.done))).toEqual(untouched);
  await page.getByRole("button", { name: t("common.undo") }).click();
  await expect(columnOf(page, statusNames.open).getByText(key)).toBeVisible();
  expect(await statusOfIssue(admin, key)).toBe(statusNames.open);
  await admin.dispose();
});

test("plan_06_a_column_the_workflow_does_not_allow_fades_and_says_why", async ({ page }) => {
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.bug;
  await takeUp(page, grip(page, key));
  // Held over the column it cannot go to: the column fades and the reason stands over its cards.
  const over = await middleOf(columnOf(page, statusNames.done));
  await page.mouse.move(over.x, over.y, { steps: 6 });
  const reason = t("board.reason-no-change", {
    key,
    from: statusNames.open,
    column: statusNames.done,
  });
  await expect(page.getByText(reason)).toBeVisible();
  await expect(headOf(page, statusNames.done)).toHaveAttribute("data-state", /faded/u);
  await expect(headOf(page, statusNames.progress)).not.toHaveAttribute("data-state", /faded/u);
  await page.mouse.up();
  // Dropped there, the card stays where it was and the column is as it was.
  await expect(page.getByText(reason)).toBeHidden();
  await expect(columnOf(page, statusNames.open).getByText(key)).toBeVisible();
});

test("plan_06_over_the_card_limit_the_title_turns_orange_and_the_card_stays", async ({ page }) => {
  await signIn(page, ada);
  await openBoard(page);
  const head = headOf(page, statusNames.progress);
  await expect(head).toContainText(t("board.count-of-limit", { count: 3, limit: progressLimit }));
  await expect(partIn(head, "board-head.title")).toHaveAttribute("data-state", /over/u);
  await expect(columnOf(page, statusNames.progress).getByText(cards.working)).toBeVisible();
  // Nothing is refused: a fourth card is taken in.
  await moveButton(page, cards.open).click();
  await page.getByRole("button", { name: statusNames.progress, exact: true }).click();
  await expect(head).toContainText(t("board.count-of-limit", { count: 4, limit: progressLimit }));
});

test("plan_06_move_to_does_what_a_drag_does", async ({ page, server }) => {
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.bug;
  const button = moveButton(page, key);
  await button.focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", { name: t("board.move-to-title", { key }) });
  await expect(window).toBeVisible();
  // The column it cannot go to is there with its reason, and cannot be chosen.
  const done = window.getByRole("button", { name: statusNames.done, exact: true });
  await expect(done).toBeDisabled();
  await expect(
    window.getByText(
      t("board.reason-no-change", { key, from: statusNames.open, column: statusNames.done }),
    ),
  ).toBeVisible();
  const progress = window.getByRole("button", { name: statusNames.progress, exact: true });
  await progress.focus();
  await page.keyboard.press("Enter");
  await expect(columnOf(page, statusNames.progress).getByText(key)).toBeVisible();
  const admin = await signInThroughApi(server, ada);
  expect(await statusOfIssue(admin, key)).toBe(statusNames.progress);
  // The keys of the card do the same: Space picks it up, the arrows choose the column, Enter drops it.
  const other = cards.other;
  await cardOf(page, other).getByRole("link").focus();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await expect(columnOf(page, statusNames.progress).getByText(other)).toBeVisible();
  expect(await statusOfIssue(admin, other)).toBe(statusNames.progress);
  await admin.dispose();
});

test("plan_06_customize_sets_columns_rows_and_card_limits_for_the_maker_only", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ada);
  await openBoard(page);
  await page.getByRole("button", { name: t("board.customize"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("board.customize-title") });
  await window.getByLabel(t("board.card-limit-of", { column: statusNames.open })).fill("1");
  await window.getByRole("button", { name: t("board.save") }).click();
  await expect(headOf(page, statusNames.open)).toContainText(
    t("board.count-of-limit", { count: 3, limit: 1 }),
  );
  // Somebody the search is shared with does not see it.
  const admin = await signInThroughApi(server, ada);
  const id = await savedIdOf(admin, boardName);
  await shareWithWeb(admin, id);
  const other = await browser.newPage({ ignoreHTTPSErrors: true });
  await other.goto(new URL("/", page.url()).href);
  await signIn(other, bob);
  await other.goto(savedSearchAddress(id));
  await expect(boardOn(other)).toBeVisible();
  await expect(other.getByRole("button", { name: t("board.customize"), exact: true })).toHaveCount(
    0,
  );
  await other.close();
  await admin.dispose();
});
