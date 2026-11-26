import { t } from "@bugaboo/messages";

import { savedSearchAddress } from "../src/shared/paths.ts";
import { ada, bob, signInThroughApi } from "./support/accounts.ts";
import {
  boardOn,
  cardOf,
  cards,
  columnOf,
  headOf,
  openBoard,
  prepareBoard,
  statusNames,
  statusOfIssue,
} from "./support/board.ts";
import { resetBoard, sharePlanBoard } from "./support/board-reset.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { letGoOn, takeUp } from "./support/hand-drag.ts";
import { partIn } from "./support/parts.ts";

// *Customize* of the board, a drag that starts on the title of a card, and the sign that the columns go
// on beyond the edge (PLAN-06, UX §4.6).

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

test("plan_06_customize_sets_columns_rows_and_card_limits_for_the_maker_only", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ada);
  await openBoard(page);
  const customize = page.getByRole("button", { name: t("board.customize"), exact: true });
  await customize.click();
  const window = page.getByRole("dialog", { name: t("board.customize-title") });
  await window.getByLabel(t("board.card-limit-of", { column: statusNames.open })).fill("1");
  await window.getByRole("button", { name: t("board.save") }).click();
  await expect(headOf(page, statusNames.open)).toContainText(
    t("board.count-of-limit", { count: 3, limit: 1 }),
  );
  // The columns come from the status groups and the rows from the people the issues are assigned to.
  await customize.click();
  await window.getByLabel(t("board.columns-from")).selectOption({ label: "Status group" });
  await window.getByLabel(t("board.rows")).selectOption({ label: "Assigned to" });
  await window.getByRole("button", { name: t("board.save") }).click();
  await expect(window).toBeHidden();
  await expect(headOf(page, t("status-group.not-started"))).toBeVisible();
  await expect(headOf(page, t("status-group.in-progress"))).toBeVisible();
  await expect(headOf(page, t("status-group.done"))).toBeVisible();
  await expect(headOf(page, statusNames.testing)).toHaveCount(0);
  await expect(partIn(page, "board-row")).toHaveCount(1);
  await expect(partIn(page, "board-row.toggle")).toContainText(
    t("board.no-row-value", { field: "Assigned to" }),
  );
  // Somebody the search is shared with does not see it.
  const admin = await signInThroughApi(server, ada);
  const id = await sharePlanBoard(admin);
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

test("plan_06_a_drag_that_starts_on_the_title_carries_the_card_and_a_click_opens_the_issue", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openBoard(page);
  const key = cards.other;
  // The title is a link; the card follows the hand all the same.
  await takeUp(page, cardOf(page, key).getByRole("link"));
  await letGoOn(page, columnOf(page, statusNames.progress));
  await expect(columnOf(page, statusNames.progress).getByText(key)).toBeVisible();
  const admin = await signInThroughApi(server, ada);
  expect(await statusOfIssue(admin, key)).toBe(statusNames.progress);
  await admin.dispose();
  // A click on it opens the issue.
  await cardOf(page, key).getByRole("link").click();
  await expect(page).toHaveURL(new RegExp(key, "u"));
});

test("plan_06_columns_wider_than_the_room_show_a_shadow_at_the_end_that_has_more", async ({
  page,
}) => {
  await signIn(page, ada);
  await openBoard(page);
  const scroll = partIn(boardOn(page), "board-scroll");
  const sizes = await scroll.evaluate((box) => ({ wide: box.scrollWidth, room: box.clientWidth }));
  expect(sizes.wide).toBeGreaterThan(sizes.room);
  // The scroll position drives the shadows: at the start only the end has one, at the end only the start.
  const opacityOf = (end: "before" | "after") =>
    scroll.evaluate((box, which) => Number(getComputedStyle(box, `::${which}`).opacity), end);
  await expect.poll(() => opacityOf("before")).toBe(0);
  await expect.poll(() => opacityOf("after")).toBeGreaterThan(0);
  await scroll.evaluate((box) => {
    box.scrollLeft = box.scrollWidth;
  });
  await expect.poll(() => opacityOf("before")).toBeGreaterThan(0);
  await expect.poll(() => opacityOf("after")).toBe(0);
});
