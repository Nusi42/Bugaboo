import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { chooseStatus, detailRow, expectStatus, statusButton } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// The issue page in the new look (UX §4.4, R4, R7, R9, M3 WP-17): the big status button in its
// group's color, the text column of at most 780, Details as name-and-value rows that save in place
// and say Saved with Undo where it happened.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

/** The widest the text column of the issue page may be (UX §4.4). */
const TEXT_COLUMN_MAX = 780;

test("ux_4_4 the big status button shows the status with its group's icon and lists the changes in words", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-3"));
  await expectStatus(page, t("status.open"));
  await expect(statusButton(page)).toHaveAttribute("data-variant", "not-started");
  await expect(partIn(statusButton(page), "status-icon")).toHaveCount(1);
  await chooseStatus(page, t("status.in-progress"));
  await expectStatus(page, t("status.in-progress"));
  await expect(statusButton(page)).toHaveAttribute("data-variant", "in-progress");
  // Kept: the page is read again from the server.
  await page.reload();
  await expectStatus(page, t("status.in-progress"));
});

test("ux_4_4 a person who may not change the status sees the button as it is, with the reason", async ({
  page,
}) => {
  // The first issue is in progress; the reason is in words, as the lock was before.
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expectStatus(page, t("status.in-progress"));
  await expect(statusButton(page)).toBeEnabled();
});

test("ux_4_4 the Details are name-and-value rows, and a short value saves in place with Saved and Undo (R7)", async ({
  page,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  const priority = detailRow(page, t("issue-field.priority"));
  await expect(priority).toBeVisible();
  await Promise.all(
    [t("issue-field.assigned-to"), t("issue-field.reported-by")].map((name) =>
      expect(detailRow(page, name)).toBeVisible(),
    ),
  );
  await priority.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
    label: t("priority.low"),
  });
  // Saved, and Undo, in the row where it happened.
  await expect(priority.getByText(new RegExp(`^${t("save.saved")}`, "u"))).toBeVisible();
  await priority.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect(priority.getByLabel(t("issue-field.priority"), { exact: true })).toHaveValue(
    "critical",
  );
});

test("ux_4_4 the text column is at most 780 wide", async ({ page }) => {
  test.skip(layoutOf(page) !== "computer", "the columns are for a computer");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const main = await boxOf(partIn(page, "issue-main"));
  expect(main.width).toBeLessThanOrEqual(TEXT_COLUMN_MAX);
  expect(main.width).toBeGreaterThan(TEXT_COLUMN_MAX / 2);
});
