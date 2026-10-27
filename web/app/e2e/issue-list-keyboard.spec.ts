import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { leaveBox } from "./support/list-keys.ts";
import { grid, groupOf, prepareLook, rowOf } from "./support/list-look.ts";

// The list with the keyboard (UX §4.2, §6.2): the grid is one Tab stop, the arrow keys move over the
// group headers and the rows, Enter on a header folds its group, Enter on a row opens its issue,
// and j and k move over the titles.

/** The word after the number of issues shown in the count line. */
const shown = t("issues.count-line", { total: 0, scope: "", shown: 0 }).split(" ").at(-1) ?? "";

/** The count line with this many issues shown. */
function shownCount(count: number): RegExp {
  return new RegExp(`${String(count)} ${shown}$`, "u");
}

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

test("ux_6_2 the arrow keys reach the group headers, Enter folds a group and opens a row", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the keyboard is the computer's way into the list");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  const running = groupOf(page, t("status-group.in-progress"));
  // The grid is one Tab stop, and it lands on its first row: the header of the first group.
  await grid(page).focus();
  await expect(running).toBeFocused();
  // Enter folds the group, and its rows are gone and not counted as shown.
  await page.keyboard.press("Enter");
  await expect(rowOf(page, webIssues.login)).toHaveCount(0);
  await expect(running).toHaveAttribute(
    "aria-label",
    t("issues.group-folded", { group: t("status-group.in-progress"), count: 1 }),
  );
  await page.keyboard.press("Enter");
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  // The arrow goes on to the first row, and Enter opens its issue.
  await page.keyboard.press("ArrowDown");
  await expect(rowOf(page, webIssues.login)).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/WEB-1/u);
});

test("ux_6_2 j and k move over the rows, as the arrow keys do", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "the keyboard is the computer's way into the list");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await leaveBox(page);
  await page.keyboard.press("j");
  await expect(groupOf(page, t("status-group.in-progress"))).toBeFocused();
  await page.keyboard.press("j");
  await expect(rowOf(page, webIssues.login)).toBeFocused();
  await page.keyboard.press("k");
  await expect(groupOf(page, t("status-group.in-progress"))).toBeFocused();
});

test("ux_6_2 a folded group takes its rows out of the count of the issues shown", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await expect(page.getByText(shownCount(3))).toBeVisible();
  await groupOf(page, t("status-group.in-progress")).click();
  await expect(page.getByText(shownCount(2))).toBeVisible();
});

test("ux_6_2 the grid is one Tab stop: Tab leaves it and Shift+Tab comes back to the row it was on", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the keyboard is the computer's way into the list");
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await grid(page).focus();
  await page.keyboard.press("ArrowDown");
  await expect(rowOf(page, webIssues.login)).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(grid(page).locator(":focus")).toHaveCount(0);
  await page.keyboard.press("Shift+Tab");
  await expect(rowOf(page, webIssues.login)).toBeFocused();
});

test("ux_4_2 the head has column headers that say their order and edges that set a width", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone has cards and no head");
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?group=-`);
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  const title = page.getByRole("columnheader", { name: t("issue-field.title"), exact: true });
  await expect(title).toHaveAttribute("aria-sort", "none");
  await title.getByRole("button").click();
  await expect(title).toHaveAttribute("aria-sort", "ascending");
  // The cells of the rows are named by the header of their column.
  await expect(
    rowOf(page, webIssues.login).getByRole("group", { name: t("issue-field.title"), exact: true }),
  ).toBeVisible();
  const edge = page.getByRole("separator", {
    name: t("issues.column-width", { column: t("issue-field.title") }),
  });
  const before = Number(await edge.getAttribute("aria-valuenow"));
  expect(before).toBeGreaterThan(0);
  await edge.focus();
  await page.keyboard.press("ArrowLeft");
  await expect(edge).not.toHaveAttribute("aria-valuenow", String(before));
});
