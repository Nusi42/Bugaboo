import { t } from "@bugaboo/messages";

import { savedSearchAddress } from "../src/shared/paths.ts";
import { ada, bob, idOf, signInThroughApi } from "./support/accounts.ts";
import { cards, columnOf, headOf, openBoard, prepareBoard, statusNames } from "./support/board.ts";
import { nextProjectName, resetBoard, saveExtraBoard } from "./support/board-reset.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { assignTo, issueOf, versionOf } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";

// The counts of a board and its rows (PLAN-06, UX §4.6): a head says how many issues a row holds, and
// pressing it folds the row.

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

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
  const rowsId = await saveExtraBoard(admin, `Rows ${nextProjectName()}`, {
    columns: null,
    rows: "assigned to",
    card_limits: [],
  });
  await page.goto(savedSearchAddress(rowsId));
  const rows = partIn(page, "board-row");
  await expect(rows).toHaveCount(2);
  // Each row's head says how many issues it holds: one for Bob, the other six of the seven.
  const bobs = rows.filter({ hasText: bob.name });
  await expect(partIn(bobs, "board-row.count")).toHaveText(t("board.count", { count: 1 }));
  const others = rows.filter({ hasNotText: bob.name });
  await expect(partIn(others, "board-row.count")).toHaveText(t("board.count", { count: 6 }));
  // Pressing a head folds the row, and again opens it.
  const toggle = partIn(bobs, "board-row.toggle");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(partIn(bobs, "board-lane")).toHaveCount(1);
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(partIn(bobs, "board-lane")).toHaveCount(0);
  await toggle.click();
  await expect(partIn(bobs, "board-lane")).toHaveCount(1);
  await admin.dispose();
});
