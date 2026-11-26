import { t } from "@bugaboo/messages";

import { savedSearchAddress } from "../src/shared/paths.ts";
import { ada, signInThroughApi } from "./support/accounts.ts";
import {
  cards,
  columnOf,
  headOf,
  manyOpenCards,
  plannedBoardId,
  prepareBoard,
  resetBoard,
  saveBoard,
  statusNames,
  webProjectId,
} from "./support/board.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, setStatusAndPriority, versionOf } from "./support/issues.ts";
import { createProject } from "./support/projects.ts";
import { giveOwnWorkflow } from "./support/workflows.ts";

// A board over a project and its sub-projects, and a long column that shows its first page and *Show
// more* (PRJ-06, PLAN-06, UX §4.6).

/** The cards of *Open* in the scene, and the ones this test adds: together they pass the first page of a column. */
const openCards = 3;
const extraCards = 25;

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

test("prj_06_a_board_across_sub_projects_merges_status_columns_by_name", async ({
  page,
  server,
}) => {
  const admin = await signInThroughApi(server, ada);
  await createProject(admin, "SUB", "Subsite", await webProjectId(admin));
  await giveOwnWorkflow(admin, "SUB", {
    name: "Subsite",
    statuses: [
      { key: "new", name: "Open", status_group: "not-started" },
      { key: "going", name: "In progress", status_group: "in-progress" },
      { key: "review", name: "Review", status_group: "in-progress" },
    ],
    changes: [
      { from: "new", to: "going", button: "Go", rules: {} },
      { from: "going", to: "review", button: "Check", rules: {} },
    ],
  });
  const made = await createIssue(admin, "SUB", "A card of the sub-project");
  await setStatusAndPriority(admin, made, await versionOf(admin, made), {
    status: "going",
    priority: "normal",
  });
  const id = await saveBoard(
    admin,
    "Across",
    { columns: null, rows: null, card_limits: [] },
    "project within WEB",
  );
  await signIn(page, ada);
  await page.goto(savedSearchAddress(id));
  // One column of the name In progress holds the cards of both workflows; the column of Review is there too.
  await expect(headOf(page, statusNames.progress)).toHaveCount(1);
  await expect(columnOf(page, statusNames.progress).getByText(made.key)).toBeVisible();
  await expect(columnOf(page, statusNames.progress).getByText(cards.working)).toBeVisible();
  await expect(headOf(page, "Review")).toHaveCount(1);
  await admin.dispose();
});

test("plan_06_a_long_column_shows_its_first_page_and_show_more", async ({ page, server }) => {
  const admin = await signInThroughApi(server, ada);
  await manyOpenCards(admin, extraCards);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(savedSearchAddress(plannedBoardId()));
  const open = columnOf(page, statusNames.open);
  const shown = open.getByRole("listitem").filter({ has: page.locator("a") });
  const more = open.getByRole("button", { name: t("board.show-more") });
  // The first page is not all of it: three cards of the scene and the new ones are more than a page.
  await expect(more).toBeVisible();
  const first = await shown.count();
  expect(first).toBeLessThan(extraCards + openCards);
  await more.click();
  await expect(shown).toHaveCount(extraCards + openCards);
  await expect(more).toHaveCount(0);
});
