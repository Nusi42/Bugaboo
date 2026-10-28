import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { addRelation, addSubIssue, BLOCKS } from "./support/relate.ts";

// The rows under the text of an issue (UX §4.4, R9, M3 WP-17): sub-issues and related issues are
// rows like those of the list, with the same tint, and the issue under another says *Part of* with
// the status as the pill of its group.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

test("ux_4_4 a sub-issue is a row like those of the list, tinted by its status group (R9)", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const parts = page.getByRole("region", { name: t("sub-issues.title") });
  await addSubIssue(page, "WEB-2");
  const row = partIn(parts, "list-row").filter({ hasText: "WEB-2" });
  // The same state as the list: WEB-2 is done.
  await expect(row).toHaveAttribute("data-state", /group-done/u);
  // And its priority edge: WEB-2 is high (M3-F173).
  await expect(row).toHaveAttribute("data-state", /priority-high/u);
  await expect(row.getByRole("img", { name: t("status.done"), exact: true })).toBeVisible();
  // The whole row opens the issue, and the button to take it out is in a cell of its own.
  await expect(row.getByRole("link")).toHaveAttribute("href", issueAddress("WEB-2"));
  await expect(
    partIn(row, "list-row.cell", "action").getByRole("button", {
      name: t("sub-issues.take-out", { key: "WEB-2" }),
      exact: true,
    }),
  ).toBeVisible();
  // The headings of the blocks are the section titles, one level under the title of the page.
  await Promise.all(
    [t("sub-issues.title"), t("related-issues.title"), t("history.title")].map((name) =>
      expect(page.getByRole("heading", { level: 2, name, exact: true })).toHaveAttribute(
        "data-part",
        "issue-section.title",
      ),
    ),
  );
});

test("ux_4_4 the issue under another says Part of with the status as the pill of its group", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  const above = page.getByText(t("issue-field.parent"), { exact: true });
  await expect(above).toBeVisible();
  // One status pill, the one of the parts of the design: WEB-1 is in progress.
  await expect(partIn(page, "status-pill")).toHaveCount(1);
  await expect(partIn(page, "status-pill")).toContainText(t("status.in-progress"));
  await expect(partIn(page, "issue-ref.status")).toHaveCount(0);
});

test("ux_4_4 a related issue is a row with what the relation means before its title", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await addRelation(page, "WEB-1", "WEB-3");
  const related = page.getByRole("region", { name: t("related-issues.title") });
  const row = partIn(related, "list-row").filter({ hasText: "WEB-3" });
  await expect(row).toContainText(BLOCKS);
  // WEB-3 has not started and has a normal priority: its row has the tint of its group and no edge.
  await expect(row).toHaveAttribute("data-state", /group-not-started/u);
  await expect(row).not.toHaveAttribute("data-state", /priority-/u);
  await expect(row.getByRole("img", { name: t("status.open"), exact: true })).toBeVisible();
});
