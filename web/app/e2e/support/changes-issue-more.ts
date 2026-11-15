import { expect, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { bob } from "./accounts.ts";
import { type ChangeInPlace, detailChange, ISSUE, issuePage } from "./change-row.ts";
import { issuePageLandmarks } from "./landmarks.ts";
import { descriptionBox, saveDescriptionButton } from "./description.ts";
import { detailRow } from "./parts.ts";
import { BLOCKS } from "./relate.ts";
import { answerTo } from "./sequence.ts";
import { issueAddress } from "../../src/shared/paths.ts";

// More changes in place of the issue page, for the table of changes (`changes-in-place.ts`): the
// people, own fields, relations, booked time and the timer.

/** The own fields the preparation of the table makes for issues: one choice, several, and a text. */
export const OWN = {
  one: { name: "Severity", choices: ["Minor", "Critical"] },
  several: { name: "Platforms", choices: ["Windows", "Mac"] },
  text: { name: "Customer" },
} as const;

/** The issue the relations of the table point to: one that is in no plan of the preparation. */
const OTHER = "WEB-3";

/** The issue the preparation makes to become a part of the issue of the table. */
export const PART = { key: "WEB-17", title: "A part in place" } as const;

/** The words of the description the table writes. */
const DESCRIPTION = "A description written in place";

/** The region of a section of the main column, by its heading. */
function section(page: Page, heading: string) {
  return page.getByRole("region", { name: heading });
}

/** The window of *Add time* of the issue of the table. */
function addTimeWindow(page: Page) {
  return page.getByRole("dialog", { name: t("time.add.title", { issue: ISSUE }) });
}

/** A change that adds something to a section of the main column, after the person filled its form. */
function sectionChange(
  heading: string,
  change: string,
  steps: Pick<ChangeInPlace, "before" | "act">,
): ChangeInPlace {
  return {
    screen: issueAddress("$issue"),
    change,
    open: issuePage,
    part: (page) => section(page, heading).first(),
    ...steps,
    landmarks: issuePageLandmarks,
  };
}

/** The changes in place of the issue page beyond the short values, the title and the status. */
export const moreIssuePageChanges: readonly ChangeInPlace[] = [
  {
    ...detailChange(t("issue-field.assigned-to"), "assigned to", async (page) => {
      await page
        .getByRole("button", { name: t("issues.assign-name", { name: bob.name }), exact: true })
        .click();
    }),
    // Opening the picker is the person's own; picking the person makes the change.
    before: async (page) => {
      await page.getByRole("button", { name: t("issues.assign-someone"), exact: true }).click();
    },
  },
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
  sectionChange(t("related-issues.title"), "related issue", {
    before: async (page) => {
      await page.getByRole("button", { name: t("related-issues.add"), exact: true }).click();
      await page.getByRole("radio", { name: BLOCKS, exact: true }).check();
      await section(page, t("related-issues.title")).getByLabel(t("common.issue-id")).fill(OTHER);
      // The person brings the button into view before pressing it: the section with its form may be higher than the
      // box of the sections that scrolls.
      await page
        .getByRole("button", { name: t("related-issues.add-it"), exact: true })
        .scrollIntoViewIfNeeded();
    },
    act: async (page) => {
      await page.getByRole("button", { name: t("related-issues.add-it"), exact: true }).click();
      await expect(
        page.getByText(t("related-issues.added", { key: ISSUE, name: BLOCKS, other: OTHER })),
      ).toBeVisible();
    },
  }),
  sectionChange(t("sub-issues.title"), "sub-issue", {
    before: async (page) => {
      const parts = section(page, t("sub-issues.title"));
      await parts.getByRole("button", { name: t("sub-issues.add"), exact: true }).click();
      await parts.getByLabel(t("common.issue-id")).fill(PART.key);
      await parts
        .getByRole("button", { name: t("sub-issues.add-it"), exact: true })
        .scrollIntoViewIfNeeded();
    },
    act: async (page) => {
      await section(page, t("sub-issues.title"))
        .getByRole("button", { name: t("sub-issues.add-it"), exact: true })
        .click();
      await expect(page.getByText(t("sub-issues.added", { key: PART.key }))).toBeVisible();
    },
  }),
  sectionChange(t("issue-field.description"), "description", {
    // Opening the editor and writing are the person's own; the person scrolls to *Save* and presses it.
    before: async (page) => {
      // The editor opens whole once the person's draft is looked for.
      const looked = answerTo(page, "GET", "/api/v1/drafts");
      await page.getByRole("button", { name: t("issues.description.edit"), exact: true }).click();
      await looked;
      await descriptionBox(page).fill(DESCRIPTION);
      await saveDescriptionButton(page).scrollIntoViewIfNeeded();
    },
    act: async (page) => {
      await saveDescriptionButton(page).click();
      await expect(descriptionBox(page)).toHaveCount(0);
      await expect(
        section(page, t("issue-field.description")).getByText(DESCRIPTION),
      ).toBeVisible();
    },
  }),
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
