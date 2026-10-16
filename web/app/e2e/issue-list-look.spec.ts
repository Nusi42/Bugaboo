import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { expectNoSidewaysScroll } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";
import {
  grid,
  openList,
  placeInGrid,
  prepareLook,
  pressMiddleOf,
  rowOf,
} from "./support/list-look.ts";

// The list of issues in the new look (UX §4.2, R1, R4, R5, R8, R9, M3 WP-17): one grid with fixed
// columns, groups by status group, a row tinted by its status group and edged by its priority,
// the status and the priority told by icon and name and not by color alone, a key to the colors,
// and a whole row that opens its issue.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

const groups = {
  running: t("status-group.in-progress"),
  waiting: t("status-group.not-started"),
  finished: t("status-group.done"),
} as const;

test("iss_01 the issues are a grid in groups by status group, each with its name and its number", async ({
  page,
}) => {
  await openList(page);
  await expect(grid(page).getByRole("rowheader")).toHaveText([
    groups.running,
    groups.waiting,
    groups.finished,
  ]);
  // The number of issues of a group stands next to its name.
  await expect(partIn(grid(page), "list-group").filter({ hasText: groups.running })).toContainText(
    "1",
  );
  // Each issue stands under the header of its group.
  const running = await placeInGrid(page, groups.running);
  const waiting = await placeInGrid(page, groups.waiting);
  const finished = await placeInGrid(page, groups.finished);
  const login = await placeInGrid(page, webIssues.login);
  const passkey = await placeInGrid(page, webIssues.passkey);
  expect(login).toBeGreaterThan(running);
  expect(login).toBeLessThan(waiting);
  expect(passkey).toBeGreaterThan(finished);
});

test("ux_08 a row says its status and its priority by icon and name, not by color alone (R5)", async ({
  page,
}) => {
  await openList(page);
  const critical = rowOf(page, webIssues.login);
  await expect(
    critical.getByRole("img", { name: t("status.in-progress"), exact: true }),
  ).toBeVisible();
  // Only Critical shows its word; its icon is there as well.
  await expect(critical.getByText(t("priority.critical"), { exact: true })).toBeVisible();
  const high = rowOf(page, webIssues.passkey);
  await expect(high.getByRole("img", { name: t("status.done"), exact: true })).toBeVisible();
  await expect(high.getByRole("img", { name: t("priority.high"), exact: true })).toBeVisible();
  await expect(high.getByText(t("priority.high"), { exact: true })).toHaveCount(0);
  const plain = rowOf(page, webIssues.typo);
  await expect(plain.getByRole("img", { name: t("status.open"), exact: true })).toBeVisible();
});

test("ux_4_2 a row is tinted by its status group and edged by its priority (R4)", async ({
  page,
}) => {
  await openList(page);
  const [critical, high, plain] = [
    rowOf(page, webIssues.login),
    rowOf(page, webIssues.passkey),
    rowOf(page, webIssues.typo),
  ];
  await expect(critical).toHaveAttribute("data-state", /group-in-progress/u);
  await expect(critical).toHaveAttribute("data-state", /priority-critical/u);
  await expect(high).toHaveAttribute("data-state", /group-done/u);
  await expect(high).toHaveAttribute("data-state", /priority-high/u);
  // Not started and a normal priority stay plain: no tint, and no state at all.
  await expect(plain).not.toHaveAttribute("data-state", /group-|priority-/u);
  // The edge is 4 wide on the rows that have a priority to show, and the plain row keeps its place
  // with an edge that is not drawn.
  await expect(critical).toHaveCSS("border-left-width", "4px");
  const none = "rgba(0, 0, 0, 0)";
  await expect(critical).not.toHaveCSS("border-left-color", none);
  await expect(plain).toHaveCSS("border-left-color", none);
});

test("ux_4_2 a key beside the list says what the row color and the left edge mean", async ({
  page,
}) => {
  await openList(page);
  const legend = page.getByRole("list", { name: t("issues.legend"), exact: true });
  await expect(legend).toBeVisible();
  await Promise.all(
    [groups.running, groups.finished, t("priority.critical"), t("priority.high")].map((word) =>
      expect(legend.getByText(word, { exact: true })).toBeVisible(),
    ),
  );
});

test("ux_4_2 the list is left-aligned: it starts where the heading starts and fills its box (R1)", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the phone has its own two-line rows");
  await openList(page);
  const heading = await boxOf(
    page.getByRole("heading", { level: 1, name: t("issues.page-title", { project: "Website" }) }),
  );
  const table = await boxOf(grid(page));
  expect(Math.abs(table.x - heading.x)).toBeLessThanOrEqual(2);
  // The title column takes what the others leave: the table is not a narrow column in the middle.
  expect(table.width).toBeGreaterThan(table.x * 2);
});

test("ux_4_2 a whole row opens its issue, and its title is not a link color (R8)", async ({
  page,
}) => {
  await openList(page);
  const row = rowOf(page, webIssues.login);
  await expect(row.getByRole("link", { name: webIssues.login })).toHaveCSS(
    "text-decoration-line",
    "none",
  );
  // A press on the issue ID, which is no link, still opens the issue.
  await pressMiddleOf(page, partIn(row, "list-row.cell", "key"));
  await expect(page).toHaveURL(new RegExp(`${issueAddress("WEB-1")}$|open=WEB-1$`, "u"));
});

test("ux_03 on a phone a row has two lines in the same order, and the page does not scroll sideways", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has two-line rows");
  await openList(page);
  const row = rowOf(page, webIssues.login);
  await expect(row).toHaveAttribute("data-variant", "phone");
  // Status, ID and priority on the first line, the title on the second: more than one line high.
  expect((await boxOf(row)).height).toBeGreaterThan(64);
  await expectNoSidewaysScroll(page);
});

test("ux_3_4 the slim list beside an issue is left-aligned, with the open issue marked and the row state", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "the slim list is for the focus view of a computer");
  await openList(page);
  await page.goto(issueAddress("WEB-1"));
  const slim = page.getByRole("table", { name: t("issues.list"), exact: true });
  const row = partIn(slim, "list-row").filter({ hasText: webIssues.login });
  await expect(row).toHaveAttribute("aria-current", "true");
  await expect(row).toHaveAttribute("data-state", /group-in-progress/u);
  const box = await boxOf(row);
  const title = await boxOf(row.getByRole("link", { name: webIssues.login }));
  const key = await boxOf(partIn(row, "list-row.cell", "key"));
  // The title starts at the start edge of the row (not centered), under the status and the ID.
  expect(title.x - box.x).toBeLessThanOrEqual(24);
  expect(key.x).toBeGreaterThan(title.x);
  const other = await boxOf(
    partIn(slim, "list-row")
      .filter({ hasText: webIssues.passkey })
      .getByRole("link", { name: webIssues.passkey }),
  );
  expect(Math.abs(other.x - title.x)).toBeLessThanOrEqual(2);
});

test("ux_4_2 pressing any cell opens the issue, and the tick box ticks without opening it (R8)", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the phone rows have other cells");
  await openList(page);
  const row = rowOf(page, webIssues.login);
  await row.locator("label").click();
  await expect(
    page.getByRole("status").filter({ hasText: t("issues.selected", { count: 1 }) }),
  ).toHaveCount(1);
  await expect(page).not.toHaveURL(/WEB-1/u);
  await pressMiddleOf(page, partIn(row, "list-row.cell", "priority"));
  await expect(page).toHaveURL(new RegExp(`${issueAddress("WEB-1")}$|open=WEB-1$`, "u"));
});
