import { expect, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { type ChangeInPlace, ISSUE } from "./change-row.ts";
import { listLandmarks, ROWS_BOX } from "./landmarks.ts";
import { columnButton, grid, tickIssue } from "./list-look.ts";
import { listViewLandmarks, timelineLandmarks } from "./screen-landmarks.ts";
import { barOf, movedLater, openTimeline, planned, pressOf } from "./timeline.ts";
import { projectIssuesAddress } from "../../src/shared/paths.ts";

// The changes in place of a list of issues and of the timeline, for the table of changes
// (`changes-in-place.ts`): ticking an issue, and the views and filters of the list.

/** The route of the list of a project, as the table of screens names it. */
const LIST = projectIssuesAddress("$project");

/** Opens the list of the project WEB, whole. */
async function openList(page: Page): Promise<void> {
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.locator("[data-part='list-scroll'] [role='row']").first()).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** The box the rows of the list scroll in: what a change of the view changes. */
function rowsBox(page: Page) {
  return page.locator(ROWS_BOX.selector);
}

/** The changes in place of the list of a project. */
export const listChanges: readonly ChangeInPlace[] = [
  {
    screen: LIST,
    change: "tick an issue",
    open: openList,
    part: rowsBox,
    act: async (page) => {
      await tickIssue(page, ISSUE);
      await expect(
        page.getByRole("status").filter({ hasText: t("issues.selected", { count: 1 }) }),
      ).toBeVisible();
    },
    landmarks: (page) => listLandmarks(page),
  },
  {
    screen: LIST,
    change: "fold a group",
    open: openList,
    part: rowsBox,
    act: async (page) => {
      const head = grid(page).locator("[data-part='list-group']").first();
      // The head says whether its group is open or folded.
      const open = (await head.getAttribute("aria-label")) ?? "";
      await head.click();
      await expect(head).not.toHaveAttribute("aria-label", open);
    },
    landmarks: () => Promise.resolve(listViewLandmarks()),
  },
  {
    screen: LIST,
    change: "sort by a column",
    // A phone has cards and no column names.
    onlyOn: "computer",
    open: openList,
    part: rowsBox,
    act: async (page) => {
      await columnButton(page, t("issue-field.title")).click();
      await expect.poll(() => new URL(page.url()).searchParams.get("s")).toContain("sort by title");
    },
    landmarks: () => Promise.resolve(listViewLandmarks()),
  },
];

/** The changes in place of the timeline: moving a bar by a key. */
export const timelineChanges: readonly ChangeInPlace[] = [
  {
    screen: "/search?show=timeline",
    change: "move a bar by a day",
    // The timeline is drawn on a computer only.
    onlyOn: "computer",
    open: async (page) => {
      await openTimeline(page);
      await page.waitForLoadState("networkidle");
    },
    part: (page) => barOf(page, planned.pressed),
    before: async (page) => {
      await pressOf(page, planned.pressed).focus();
    },
    act: async (page) => {
      await page.keyboard.press("ArrowRight");
      const said = movedLater(planned.pressed, t("timeline.amount-days", { count: 1 }));
      await expect(page.getByText(said)).toHaveCount(1);
    },
    landmarks: (page, part) => timelineLandmarks(page, part),
  },
];
