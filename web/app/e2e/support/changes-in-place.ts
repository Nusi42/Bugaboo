import { expect, type Locator, type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { openIssueWithDetails } from "./flows.ts";
import { chooseStatus, rowSaved, statusButton } from "./issues.ts";
import { issuePageLandmarks, listLandmarks, type ScreenLandmarks } from "./landmarks.ts";
import { tickIssue } from "./list-look.ts";
import { detailRow, partIn } from "./parts.ts";
import { serverWord } from "./server-words.ts";
import { issueAddress, projectIssuesAddress } from "../../src/shared/paths.ts";

/**
 * The table of the changes a person makes in place, screen by screen (UX §6.4, M8 WP-13): each says
 * where it is made, the part it changes, what the person does before it (opening a box is theirs),
 * and the change itself, which ends when what the person waits for is on the screen. The measure of
 * movement runs over every row of it (`nothing-moves-changes.spec.ts`); a change that a screen offers
 * and the table lacks is one the pass does not hold.
 */
export interface ChangeInPlace {
  /** The route of the screen, as the table of screens names it. */
  readonly screen: string;
  /** What the person changes, in words of the glossary. */
  readonly change: string;
  /** Opens the screen, whole. */
  readonly open: (page: Page) => Promise<void>;
  /** The part the change is made in. */
  readonly part: (page: Page) => Locator;
  /** What the person does before the change (it is theirs, and not measured). */
  readonly before?: (page: Page) => Promise<void>;
  /** The change, until what the person waits for is on the screen. */
  readonly act: (page: Page) => Promise<void>;
  /** The landmarks of the screen around the part. */
  readonly landmarks: (page: Page, part: Locator) => Promise<ScreenLandmarks>;
}

/** The issue the changes of an issue page are made on: one the preparation of the tests makes. */
const ISSUE = "WEB-1";

/** The issue the change of following is made on: one the person neither reported nor commented on. */
const NOT_FOLLOWED = "WEB-3";

/**
 * Opens an issue with its Details, once every first read of the page has answered: a change is
 * measured on a whole page. (WebKit on Windows takes 4 to 11 s for those reads, M8-F11; measured
 * from the moment they all answered, its changes move nothing, M8-F127.)
 */
async function openIssue(page: Page, issue: string): Promise<void> {
  await openIssueWithDetails(page, issue);
  await expect(page.locator('[data-part="history.entry"]').first()).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** Opens the issue of the changes with its Details. */
function issuePage(page: Page): Promise<void> {
  return openIssue(page, ISSUE);
}

/** A change of a row of the Details of the issue page, which says *Saved* in its row when it is made. */
function detailChange(
  name: string,
  change: string,
  act: (page: Page) => Promise<void>,
): ChangeInPlace {
  return {
    screen: issueAddress("$issue"),
    change,
    open: issuePage,
    part: (page) => detailRow(page, name),
    act: async (page) => {
      await act(page);
      await expect(rowSaved(page, name)).toBeVisible();
    },
    landmarks: issuePageLandmarks,
  };
}

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

/** The changes in place of the issue page. */
const issuePageChanges: readonly ChangeInPlace[] = [
  detailChange(t("issue-field.priority"), "priority", async (page) => {
    await page.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
      label: t("priority.high"),
    });
  }),
  detailChange(t("issue-field.start-date"), "start date", async (page) => {
    const box = page.getByLabel(t("issue-field.start-date"), { exact: true });
    await box.fill("2026-11-20");
    await box.blur();
  }),
  detailChange(t("issue-field.due-date"), "due date", async (page) => {
    const box = page.getByLabel(t("issue-field.due-date"), { exact: true });
    await box.fill("2026-12-20");
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
    landmarks: issuePageLandmarks,
  },
];

/** The changes in place of the list of a project. */
const listChanges: readonly ChangeInPlace[] = [
  {
    screen: projectIssuesAddress("$project"),
    change: "tick an issue",
    open: async (page) => {
      await page.goto(projectIssuesAddress("WEB"));
      await expect(page.locator("[data-part='list-scroll'] [role='row']").first()).toBeVisible();
      await page.waitForLoadState("networkidle");
    },
    part: (page) => page.locator("[data-part='list-scroll']"),
    act: async (page) => {
      await tickIssue(page, ISSUE);
      await expect(
        page.getByRole("status").filter({ hasText: t("issues.selected", { count: 1 }) }),
      ).toBeVisible();
    },
    landmarks: (page) => listLandmarks(page),
  },
];

/** Every change in place of the table. */
export const changesInPlace: readonly ChangeInPlace[] = [...issuePageChanges, ...listChanges];
