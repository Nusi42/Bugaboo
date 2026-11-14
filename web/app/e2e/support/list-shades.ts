import { expect, type Locator, type Page } from "@playwright/test";

import { layoutOf } from "./layout.ts";
import { shadowsOf } from "./windows.ts";

// The shades at the ends of the box of the rows of a list, as each layout draws them (UX §4.2, §6.5).

export { shadowsOf } from "./windows.ts";

/**
 * The shades of the list in the middle of its rows (UX §4.2, §6.5). On a computer the head of the list folds over the
 * rows and the head of the columns, with its line, marks the start edge: a shade there would stand over the head. Where
 * nothing folds (a tablet, a phone) both ends are shaded.
 */
const SHADES_IN_THE_MIDDLE = {
  computer: { start: 0, end: 1 },
  tablet: { start: 1, end: 1 },
  phone: { start: 1, end: 1 },
} as const;

/** How wide the line under the head of the columns is drawn; 0 where the list has no head of its columns (a phone). */
function lineUnderHead(page: Page): Promise<number> {
  return page.evaluate(() => {
    const head = document.querySelector("[data-part~='list-head'] [data-part~='list-row']");
    return head === null ? 0 : Number(getComputedStyle(head).borderBottomWidth.replace("px", ""));
  });
}

/** Where the start of the rows has no shade, the line of the head of the columns marks it. */
const START_EDGE_LINE = {
  computer: async (page: Page) => {
    expect(await lineUnderHead(page)).toBeGreaterThanOrEqual(1);
  },
  tablet: () => Promise.resolve(),
  phone: () => Promise.resolve(),
};

/** The list of issues scrolled to the middle of its rows shades its ends as its layout does, and marks a start without a shade. */
export async function expectShadesInTheMiddle(page: Page, rows: Locator): Promise<void> {
  await expect.poll(() => shadowsOf(rows)).toEqual(SHADES_IN_THE_MIDDLE[layoutOf(page)]);
  await START_EDGE_LINE[layoutOf(page)](page);
}
