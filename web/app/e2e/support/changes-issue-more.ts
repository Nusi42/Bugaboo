import { expect, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { type ChangeInPlace, detailChange, ISSUE, issuePage } from "./change-row.ts";
import { issuePageLandmarks } from "./landmarks.ts";
import { detailRow } from "./parts.ts";
import { issueAddress } from "../../src/shared/paths.ts";

// More changes in place of the issue page, for the table of changes (`changes-in-place.ts`): own
// fields, booked time and the timer.

/** The own fields the preparation of the table makes for issues: one choice, several, and a text. */
export const OWN = {
  one: { name: "Severity", choices: ["Minor", "Critical"] },
  several: { name: "Platforms", choices: ["Windows", "Mac"] },
  text: { name: "Customer" },
} as const;

/** The window of *Add time* of the issue of the table. */
function addTimeWindow(page: Page) {
  return page.getByRole("dialog", { name: t("time.add.title", { issue: ISSUE }) });
}

/** The changes in place of the issue page beyond the short values, the title and the status. */
export const moreIssuePageChanges: readonly ChangeInPlace[] = [
  detailChange(OWN.one.name, "own field with one choice", async (page) => {
    await page.getByLabel(OWN.one.name, { exact: true }).selectOption({ label: "Critical" });
  }),
  detailChange(OWN.several.name, "own field with several choices", async (page) => {
    await detailRow(page, OWN.several.name).getByText("Windows", { exact: true }).click();
  }),
  {
    ...detailChange(OWN.text.name, "own field with a text", async (page) => {
      await page.getByLabel(OWN.text.name, { exact: true }).press("Enter");
    }),
    before: async (page) => {
      await page.getByLabel(OWN.text.name, { exact: true }).fill("Acme");
    },
  },
  {
    screen: issueAddress("$issue"),
    change: "time booked by hand",
    open: issuePage,
    part: (page) => detailRow(page, t("issue-field.time-spent")),
    before: async (page) => {
      await page.getByRole("button", { name: t("time.add.button"), exact: true }).click();
      await addTimeWindow(page).getByLabel(t("time.entry.length"), { exact: true }).fill("1:30");
    },
    act: async (page) => {
      await addTimeWindow(page)
        .getByRole("button", { name: t("time.add.save"), exact: true })
        .click();
      await expect(addTimeWindow(page)).toHaveCount(0);
      await expect(
        detailRow(page, t("issue-field.time-spent")).getByText("1h 30m", { exact: true }),
      ).toBeVisible();
    },
    landmarks: issuePageLandmarks,
  },
  {
    screen: issueAddress("$issue"),
    change: "timer started",
    open: issuePage,
    part: (page) => detailRow(page, t("issue-field.time-spent")),
    act: async (page) => {
      await page.getByRole("button", { name: t("time.timer.start"), exact: true }).click();
      await expect(detailRow(page, t("time.timer.row"))).toBeVisible();
    },
    landmarks: issuePageLandmarks,
  },
];
