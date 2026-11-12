import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { type Theme } from "./checks.ts";
import { newIssueDialog, openNewIssueDialog } from "./issues.ts";
import { layoutOf } from "./layout.ts";
import { shootWindowAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";
import { moreOf, swipe, swipeClosed, swipedActions } from "./swipe.ts";
import { swipedLists } from "./swipe-lists.ts";
import { projectIssuesAddress } from "../../src/shared/paths.ts";

/** The folder of the pictures of touch everywhere (UX-02), inside the screenshot folder. */
export const TOUCH_PICTURES = "M8/touch";

/** Waits until what slides in has stopped: every animation in time that ends has ended (a pulse never ends). */
async function stopped(page: Page): Promise<void> {
  await page.evaluate(async () => {
    const ending: Promise<unknown>[] = [];
    for (const animation of document.getAnimations()) {
      // An animation of the scroll position (the shadows at the ends of a box) never ends; only those of time do.
      const ends =
        animation.timeline === document.timeline &&
        animation.effect?.getComputedTiming().iterations !== Number.POSITIVE_INFINITY;
      if (ends) {
        ending.push(animation.finished);
      }
    }
    await Promise.all(ending);
  });
}

/** Shoots the window once it has stopped, named for what it shows, the layout and the theme. */
async function shootAs(page: Page, name: string, theme: Theme): Promise<void> {
  await stopped(page);
  await shootWindowAs(page, `${name}-${layoutOf(page)}`, TOUCH_PICTURES, theme.id);
}

/** The open side of a swipe over the first row of each list, and its *More* open as a sheet. */
async function swipePictures(page: Page, theme: Theme): Promise<void> {
  await inOrder(
    swipedLists.map((list, at) => async () => {
      const row = await list.open(page);
      const towards = list.swipes[0] ?? "start";
      await swipe(page, row, towards);
      await expect(swipedActions(row).first()).toBeVisible();
      await shootAs(page, `swipe-${String(at + 1)}-open`, theme);
      await swipeClosed(page, row, towards);
      await moreOf(row).tap();
      await expect(page.getByRole("dialog")).toBeVisible();
      await shootAs(page, `swipe-${String(at + 1)}-more`, theme);
      await page
        .getByRole("dialog")
        .getByRole("button", { name: t("common.cancel") })
        .tap();
    }),
  );
}

/** The footer of the new-issue form at the end of its scrolling part (M6-F121). */
async function formFooterPicture(page: Page, theme: Theme): Promise<void> {
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  await newIssueDialog(page)
    .getByRole("button", { name: t("common.cancel"), exact: true })
    .scrollIntoViewIfNeeded();
  await shootAs(page, "form-footer", theme);
}

/** The pictures of what changed on a phone: the swipes with their *More*, and the footer of a form. */
export async function changedPictures(page: Page, theme: Theme): Promise<void> {
  await swipePictures(page, theme);
  await formFooterPicture(page, theme);
}
