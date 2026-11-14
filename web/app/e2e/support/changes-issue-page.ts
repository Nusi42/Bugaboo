import { expect, type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import {
  type ChangeInPlace,
  detailChange,
  issuePage,
  NOT_FOLLOWED,
  openIssue,
} from "./change-row.ts";
import { chooseStatus, statusButton } from "./issues.ts";
import { issuePageLandmarks, type ScreenLandmarks } from "./landmarks.ts";
import { detailRow, partIn } from "./parts.ts";
import { serverWord } from "./server-words.ts";
import { dayFromToday } from "./timeline.ts";
import { issueAddress } from "../../src/shared/paths.ts";

// The changes in place of the issue page, for the table of changes (`changes-in-place.ts`).

/** The label the preparation makes for the change of labels. */
export const LABEL = "frontend";

/** The title the table gives the issue. */
const NEW_TITLE = "A title changed in place";

/** The words of a comment made in the table. */
const COMMENT = "A comment in place";

/** *Add comment* under the box of a comment (a phone has another *Comment* in its bar). */
function commentButton(page: Page): Locator {
  const box = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  return page
    .locator("form")
    .filter({ has: box })
    .getByRole("button", { name: t("comments.add"), exact: true });
}

/**
 * The landmarks of the issue page for opening a box that takes the place of a text (the title): it
 * stands where the text stood, at the same size, so nothing grows and nothing is pushed.
 */
async function openingMovesNothing(page: Page, part: Locator): Promise<ScreenLandmarks> {
  const around = await issuePageLandmarks(page, part);
  return { ...around, grows: [], follows: [] };
}

/** The changes in place of the issue page. */
export const issuePageChanges: readonly ChangeInPlace[] = [
  detailChange(t("issue-field.priority"), "priority", async (page) => {
    await page.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
      label: t("priority.high"),
    });
  }),
  detailChange(t("issue-field.start-date"), "start date", async (page) => {
    const box = page.getByLabel(t("issue-field.start-date"), { exact: true });
    await box.fill(dayFromToday(2));
    await box.blur();
  }),
  detailChange(t("issue-field.due-date"), "due date", async (page) => {
    const box = page.getByLabel(t("issue-field.due-date"), { exact: true });
    await box.fill(dayFromToday(30));
    await box.blur();
  }),
  detailChange(t("labels.labels-heading"), "labels", async (page) => {
    await page.getByLabel(t("labels.add-label"), { exact: true }).selectOption({ label: LABEL });
    await page.getByRole("button", { name: t("labels.add-label-button"), exact: true }).click();
  }),
  {
    ...detailChange(t("labels.tags-heading"), "tags", async (page) => {
      const tags = detailRow(page, t("labels.tags-heading"));
      await tags.getByRole("button", { name: t("labels.add-tag-button"), exact: true }).click();
    }),
    // Opening the box and typing the name are the person's own.
    before: async (page) => {
      const tags = detailRow(page, t("labels.tags-heading"));
      await tags.getByRole("button", { name: t("labels.add-tag-button"), exact: true }).click();
      await tags.getByLabel(t("labels.add-tag"), { exact: true }).fill("in-place");
    },
  },
  {
    screen: issueAddress("$issue"),
    change: "status",
    open: issuePage,
    part: (page) => partIn(page, "issue-status-line"),
    act: async (page) => {
      await chooseStatus(page, serverWord("workflow-change.start-working"));
      await expect(statusButton(page)).toContainText(t("status.in-progress"));
    },
    landmarks: issuePageLandmarks,
  },
  {
    screen: issueAddress("$issue"),
    change: "open the box of the title",
    open: issuePage,
    part: (page) => partIn(page, "issue-title"),
    act: async (page) => {
      await partIn(page, "issue-title").getByRole("button").click();
      await expect(partIn(page, "issue-title").getByRole("textbox")).toBeFocused();
    },
    landmarks: openingMovesNothing,
  },
  {
    screen: issueAddress("$issue"),
    change: "open the box of a tag",
    open: issuePage,
    part: (page) => detailRow(page, t("labels.tags-heading")),
    act: async (page) => {
      const tags = detailRow(page, t("labels.tags-heading"));
      await tags.getByRole("button", { name: t("labels.add-tag-button"), exact: true }).click();
      await expect(tags.getByLabel(t("labels.add-tag"), { exact: true })).toBeVisible();
    },
    // The box with its buttons is what the action opens: the row grows by it and pushes what stands
    // under it once, by exactly that; opening adds no line to the History.
    landmarks: (page, part) => issuePageLandmarks(page, part, false),
  },
  {
    screen: issueAddress("$issue"),
    change: "title",
    open: issuePage,
    part: (page) => partIn(page, "issue-title"),
    // Opening the box of the title and typing are the person's own; Enter makes the change.
    before: async (page) => {
      await partIn(page, "issue-title").getByRole("button").click();
      await partIn(page, "issue-title").getByRole("textbox").fill(NEW_TITLE);
    },
    act: async (page) => {
      await partIn(page, "issue-title").getByRole("textbox").press("Enter");
      await expect(page.getByRole("heading", { level: 1, name: NEW_TITLE })).toBeVisible();
    },
    landmarks: issuePageLandmarks,
  },
  {
    screen: issueAddress("$issue"),
    change: "comment",
    open: issuePage,
    part: (page) => page.getByRole("region", { name: t("history.title") }).first(),
    before: async (page) => {
      await page.getByRole("textbox", { name: t("comments.write"), exact: true }).fill(COMMENT);
      // The person scrolls to the button before pressing it.
      await commentButton(page).scrollIntoViewIfNeeded();
    },
    act: async (page) => {
      await commentButton(page).click();
      await expect(page.getByText(COMMENT)).toBeVisible();
    },
    landmarks: issuePageLandmarks,
  },
  {
    screen: issueAddress("$issue"),
    change: "follow",
    open: (page) => openIssue(page, NOT_FOLLOWED),
    part: (page) => detailRow(page, t("followers.name")),
    act: async (page) => {
      await page.getByRole("button", { name: t("followers.follow"), exact: true }).click();
      await expect(
        page.getByRole("button", { name: t("followers.stop"), exact: true }),
      ).toBeVisible();
    },
    // Following adds no line to the History.
    landmarks: (page, part) => issuePageLandmarks(page, part, false),
  },
];
