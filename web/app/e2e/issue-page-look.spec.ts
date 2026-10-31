import { t } from "@bugaboo/messages";
import { type Locator } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import {
  detailRow,
  expectStatus,
  issueNote,
  startWorking,
  statusButton,
} from "./support/issues.ts";
import { prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { inEveryLook, resolved } from "./support/styles.ts";

// The issue page in the new look (UX §4.4, R4, R7, R9, M3 WP-17): the big status button in its
// group's color, Details as name-and-value rows that save in place
// and say Saved with Undo where it happened. A thin line, in from both sides, stands between the
// sections of the main column.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

test("ux_4_4 the big status button shows the status with its group's icon and lists the changes in words", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-3"));
  await expectStatus(page, t("status.open"));
  await expect(statusButton(page)).toHaveAttribute("data-variant", "not-started");
  await expect(partIn(statusButton(page), "status-icon")).toHaveCount(1);
  await startWorking(page);
  await expectStatus(page, t("status.in-progress"));
  await expect(statusButton(page)).toHaveAttribute("data-variant", "in-progress");
  // Kept: the page is read again from the server, once the server has the change (the button shows
  // it at once, before the answer comes).
  await expect(issueNote(page)).toBeVisible();
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

/** The line drawn before a section of the main column: how thick it is and how far in from each side. */
function lineOf(section: Locator): Promise<{ width: string; start: string; end: string }> {
  return section.evaluate((node) => {
    const line = getComputedStyle(node, "::before");
    return {
      width: line.content === "none" ? "0px" : line.borderTopWidth,
      start: line.marginInlineStart,
      end: line.marginInlineEnd,
    };
  });
}

test("ux_4_4 a thin line, in from both sides, stands between the sections of the main column and not in the Details", async ({
  page,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  const sections = partIn(page, "issue-main").locator(
    ":scope > [data-part='issue-main.section']:visible",
  );
  // The description, the sub-issues, the related issues, the files and the history.
  await expect(sections).toHaveCount(5);
  const inset = await resolved(page, "margin-inline-start", "var(--inset-section-line)");
  const lines = await Promise.all((await sections.all()).map((section) => lineOf(section)));
  expect(lines[0]?.width, "no line above the first section").toBe("0px");
  for (const line of lines.slice(1)) {
    expect(line).toEqual({ width: "1px", start: inset, end: inset });
  }
  await expect(
    partIn(page, "issue-details").locator("[data-part='issue-main.section']"),
  ).toHaveCount(0);
  // The line has the color of the borders of the theme, in every theme.
  await inEveryLook(page, async (look) => {
    const border = await resolved(page, "border-top-color", "var(--border)");
    const seen = await sections
      .nth(1)
      .evaluate((node) => getComputedStyle(node, "::before").borderTopColor);
    expect(seen, look).toBe(border);
  });
});
