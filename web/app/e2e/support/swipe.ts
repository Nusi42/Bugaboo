import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { partIn } from "./parts.ts";
import { inOrder } from "./sequence.ts";
import { boxOf } from "./windows.ts";

/** The way a finger moves over a row: towards the start edge (to the left) or towards the end edge. */
export type Towards = "start" | "end";

/** How many steps a swipe moves in: a finger is seen at several places on its way, as on a real screen. */
const SWIPE_STEPS = 8;

/** How much of the width of a row a swipe crosses: well past the distance that opens a side. */
const SWIPE_SHARE = 0.5;

/** Where on the row the finger starts: a quarter of its width in from the edge it moves away from. */
const START_SHARE = 0.25;

/**
 * Moves a finger sideways over a row, as a person swipes on a phone: touches the screen, moves in steps, lets go. The
 * browser makes the pointer events of a finger from it (Chromium, through its own protocol: Playwright has no swipe).
 */
export async function swipe(page: Page, row: Locator, towards: Towards): Promise<void> {
  const box = await boxOf(row);
  const y = box.y + box.height / 2;
  const sign = towards === "start" ? -1 : 1;
  const from = sign < 0 ? box.x + box.width * (1 - START_SHARE) : box.x + box.width * START_SHARE;
  const across = sign * box.width * SWIPE_SHARE;
  const session = await page.context().newCDPSession(page);
  const touch = (type: "touchStart" | "touchMove" | "touchEnd", x: number) =>
    session.send("Input.dispatchTouchEvent", {
      type,
      touchPoints: type === "touchEnd" ? [] : [{ x, y }],
    });
  await touch("touchStart", from);
  await inOrder(
    Array.from(
      { length: SWIPE_STEPS },
      (_, step) => () => touch("touchMove", from + (across * (step + 1)) / SWIPE_STEPS),
    ),
  );
  await touch("touchEnd", from + across);
  await session.detach();
}

/** The actions a swipe brought in over a row, where they lie. */
export function swipeStrip(row: Locator): Locator {
  return partIn(row, "swipe-actions");
}

/** The actions a swipe brought in over a row, in their order. */
export function swipedActions(row: Locator): Locator {
  return swipeStrip(row).getByRole("button");
}

/** Swipes the open side of a row closed again: the other way. */
export async function swipeClosed(page: Page, row: Locator, opened: Towards): Promise<void> {
  await swipe(page, row, opened === "start" ? "end" : "start");
  await expect(swipeStrip(row)).toHaveCount(0);
}

/** The names of the actions a swipe towards `towards` brings in; the side is closed again afterwards. */
export async function actionsOfSwipe(
  page: Page,
  row: Locator,
  towards: Towards,
): Promise<string[]> {
  await swipe(page, row, towards);
  await expect(swipedActions(row).first()).toBeVisible();
  const names = await swipedActions(row).allInnerTexts();
  await swipeClosed(page, row, towards);
  return names.map((name) => name.trim());
}

/** The button *More* of a row, which holds every action of its swipes. */
export function moreOf(row: Locator): Locator {
  return partIn(row, "row-more").getByRole("button");
}

/** The names of the actions in a row's *More*, read from its sheet, which is closed again with *Cancel*. */
export async function actionsOfMore(page: Page, row: Locator): Promise<string[]> {
  const more = moreOf(row);
  const name = (await more.getAttribute("aria-label")) ?? "";
  await more.click();
  const sheet = page.getByRole("dialog", { name, exact: true });
  const names = await partIn(sheet, "action-menu").getByRole("button").allInnerTexts();
  await sheet.getByRole("button", { name: t("common.cancel"), exact: true }).click();
  await expect(sheet).toBeHidden();
  return names.map((each) => each.trim());
}
