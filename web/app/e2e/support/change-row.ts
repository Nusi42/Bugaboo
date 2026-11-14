import { expect, type Locator, type Page } from "@playwright/test";

import { openIssueWithDetails } from "./flows.ts";
import { issuePageLandmarks, type ScreenLandmarks } from "./landmarks.ts";
import { detailRow } from "./parts.ts";
import { rowSaved } from "./saved.ts";
import { issueAddress } from "../../src/shared/paths.ts";
import type { LayoutMode } from "../../../packages/ui/src/layout.ts";

/**
 * A row of the table of the changes a person makes in place (UX §6.4, M8 WP-13): where it is made,
 * the part it changes, what the person does before it (opening a box is theirs), and the change
 * itself, which ends when what the person waits for is on the screen. The measure of movement runs
 * over every row (`nothing-moves-changes.spec.ts`); a change that a screen offers and the table
 * lacks is one the pass does not hold.
 */
export interface ChangeInPlace {
  /** The route of the screen, as the table of screens names it. */
  readonly screen: string;
  /** What the person changes, in words of the glossary. */
  readonly change: string;
  /** The one layout the screen offers the change on, when it is not every layout (the timeline). */
  readonly onlyOn?: LayoutMode;
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

/**
 * The issue the changes of an issue page are made on: the one the preparation of the table makes
 * after the scene of the timeline (WEB-1 to WEB-15), untouched by it.
 */
export const ISSUE = "WEB-16";

/** The title of that issue. */
export const ISSUE_TITLE = "Changed in place";

/** The issue the change of following is made on: one the person neither reported nor commented on. */
export const NOT_FOLLOWED = "WEB-3";

/**
 * Opens an issue with its Details, once every first read of the page has answered: a change is
 * measured on a whole page. (WebKit on Windows takes 4 to 11 s for those reads, M8-F11; measured
 * from the moment they all answered, its changes move nothing, M8-F127.)
 */
export async function openIssue(page: Page, issue: string): Promise<void> {
  await openIssueWithDetails(page, issue);
  await expect(page.locator('[data-part="history.entry"]').first()).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** Opens the issue of the changes with its Details. */
export function issuePage(page: Page): Promise<void> {
  return openIssue(page, ISSUE);
}

/** A change of a row of the Details of the issue page, which says *Saved* in its row when it is made. */
export function detailChange(
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
