import { t } from "@bugaboo/messages";

import { ada, bob, signInThroughApi } from "./support/accounts.ts";
import { gripOf, cards, columnOf, openBoard, statusNames, statusOfIssue } from "./support/board.ts";
import { prepareBoardThatAsks, resetBoard } from "./support/board-reset.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { letGoOn, takeUp } from "./support/hand-drag.ts";
import { conflictNote, keepTheirs } from "./support/saved.ts";
import { changeIssue } from "./support/timeline.ts";

// A drop that asks for a value (PLAN-06, UX §4.6): the window of the issue page opens, and cancelling it leaves the card.

test.use({ preparation: { run: prepareBoardThatAsks } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

test("plan_06_a_drop_that_asks_for_a_value_opens_the_window_of_the_issue_page", async ({
  page,
  server,
}) => {
  const admin = await signInThroughApi(server, ada);
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.open;
  await takeUp(page, gripOf(page, key));
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

test("plan_06_a_drop_that_asks_for_a_value_asks_keep_bens_change_or_use_mine_when_he_changed_it_first", async ({
  page,
  server,
}) => {
  const admin = await signInThroughApi(server, ada);
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.open;
  const ben = await signInThroughApi(server, bob);
  await changeIssue(ben, key, { priority: "high" });
  await takeUp(page, gripOf(page, key));
  await letGoOn(page, columnOf(page, statusNames.progress));
  // The window asks as the issue page does; sending it finds that Ben came first, and the page asks.
  const window = page.getByRole("dialog", { name: /Start working/u });
  await window.getByRole("button", { name: t("issues.assign-someone") }).click();
  const picker = page.getByRole("dialog", { name: t("issue-field.assigned-to") });
  await picker.getByRole("button", { name: new RegExp(`Assign ${bob.name}`, "u") }).click();
  await picker.getByRole("button", { name: t("common.done"), exact: true }).click();
  await window.getByRole("button", { name: /Start working/u }).click();
  await expect(window).toBeHidden();
  const mine = conflictNote(page);
  await expect(mine).toBeVisible();
  await expect(keepTheirs(page, bob.name)).toBeVisible();
  await mine.click();
  await expect(columnOf(page, statusNames.progress).getByText(key)).toBeVisible();
  expect(await statusOfIssue(admin, key)).toBe(statusNames.progress);
  await ben.dispose();
  await admin.dispose();
});
