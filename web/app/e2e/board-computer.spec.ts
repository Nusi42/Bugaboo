import { t } from "@bugaboo/messages";

import { ada, signInThroughApi } from "./support/accounts.ts";
import {
  actionsOf,
  cardOf,
  cards,
  columnOf,
  gripOf,
  headOf,
  openBoard,
  openMoveTo,
  prepareBoard,
  progressLimit,
  statusNames,
  statusOfIssue,
} from "./support/board.ts";
import { resetBoard } from "./support/board-reset.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { middleOf } from "./support/geometry.ts";
import { letGoOn, takeUp } from "./support/hand-drag.ts";
import { expectStill, landmarkOf, movementWhile } from "./support/movement.ts";
import { partIn } from "./support/parts.ts";

// The board on a computer (PLAN-06, UX §4.6): a saved search shown as columns of cards, a drag that
// changes the status through the workflow, the columns a card cannot go to, the card limit, and
// *Move to …* with the keyboard only.

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

test("plan_06_dragging_a_card_changes_its_status_and_undo_takes_it_back", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.open;
  // Every head and the card of a column nobody touches stand still from the take-up to the drop and
  // the read that follows; only the card that moves is left out.
  const still = await Promise.all([
    ...Object.values(statusNames).map((name) => landmarkOf(headOf(page, name), `head ${name}`)),
    landmarkOf(cardOf(page, cards.done), "card of Done"),
  ]);
  const dragging = await movementWhile(page, still, async () => {
    await takeUp(page, gripOf(page, key));
    // Held over a column the card cannot go to, which fades and says why, then dropped on another.
    const over = await middleOf(columnOf(page, statusNames.testing));
    await page.mouse.move(over.x, over.y, { steps: 6 });
    await expect(headOf(page, statusNames.testing)).toHaveAttribute("data-state", /faded/u);
    await letGoOn(page, columnOf(page, statusNames.progress));
    await expect(columnOf(page, statusNames.progress).getByText(key)).toBeVisible();
  });
  expectStill(dragging);
  await expect(
    page.getByText(t("board.moved", { key, column: statusNames.progress })),
  ).toBeVisible();
  const admin = await signInThroughApi(server, ada);
  expect(await statusOfIssue(admin, key)).toBe(statusNames.progress);
  await page.getByRole("button", { name: t("common.undo") }).click();
  await expect(columnOf(page, statusNames.open).getByText(key)).toBeVisible();
  expect(await statusOfIssue(admin, key)).toBe(statusNames.open);
  await admin.dispose();
});

test("plan_06_a_column_the_workflow_does_not_allow_fades_and_says_why", async ({ page }) => {
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.bug;
  await takeUp(page, gripOf(page, key));
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
  const window = await openMoveTo(page, cards.open);
  await window.getByRole("button", { name: statusNames.progress, exact: true }).click();
  await expect(head).toContainText(t("board.count-of-limit", { count: 4, limit: progressLimit }));
});

test("plan_06_move_to_does_what_a_drag_does", async ({ page, server }) => {
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.bug;
  // The menu button of the card, then its first entry, with the keyboard only.
  await actionsOf(page, key).focus();
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: t("board.move-to"), exact: true }).focus();
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
