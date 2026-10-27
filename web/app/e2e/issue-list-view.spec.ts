import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import {
  columnButton,
  grid,
  groupOf,
  openList,
  prepareLook,
  titlesInOrder,
} from "./support/list-look.ts";

// The issue list of UX design section 4.2 (M5 WP-08): sorted by a click on a column name, grouped
// by a field, with the choice kept in the address.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

/** The list with no grouping, so the order of the rows is the order of the search. */
async function openUngrouped(page: Page): Promise<void> {
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?group=-`);
  await expect(grid(page).getByRole("link", { name: webIssues.login })).toBeVisible();
}

const title = t("issue-field.title");

test("cfg_02 sorting by a column writes sort by into the search", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone has cards and no column names");
  await openUngrouped(page);
  await columnButton(page, title).click();
  await expect.poll(() => new URL(page.url()).searchParams.get("s")).toContain("sort by title");
  await expect
    .poll(() => titlesInOrder(page))
    .toEqual([webIssues.login, webIssues.passkey, webIssues.typo]);
  // A second press turns the order around, and the search says so.
  await columnButton(page, title).click();
  await expect.poll(() => new URL(page.url()).searchParams.get("s")).toContain("Z to A");
  await expect
    .poll(() => titlesInOrder(page))
    .toEqual([webIssues.typo, webIssues.passkey, webIssues.login]);
});

test("cfg_02 group by a field shows each group with its count", async ({ page }) => {
  await openList(page);
  await page
    .getByLabel(t("issues.group-by"), { exact: true })
    .selectOption({ label: t("issue-field.priority") });
  await expect(page).toHaveURL(/group=priority/u);
  await Promise.all(
    (["critical", "high", "normal"] as const).map((word) =>
      expect(groupOf(page, t(`priority.${word}`))).toContainText("1"),
    ),
  );
  // Folding a group takes its rows away and loads nothing.
  await groupOf(page, t("priority.critical")).getByRole("button").click();
  await expect(grid(page).getByRole("link", { name: webIssues.login })).toHaveCount(0);
  await expect(groupOf(page, t("priority.high"))).toContainText("1");
});
