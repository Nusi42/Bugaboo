import { expect, type Locator } from "@playwright/test";

import { part } from "../../../packages/ui/src/tokens.ts";

// The end of the box of the rows of a list, and the bar of the ticked issues over its foot.

/**
 * Scrolls the box of the rows of a list to its end, again until it stays there: the list loads more
 * and measures the rows it draws as it is scrolled, so its end moves while it is reached.
 */
export async function scrollRowsToTheEnd(scroll: Locator): Promise<void> {
  await expect
    .poll(() =>
      scroll.evaluate(
        (box) =>
          new Promise<number>((done) => {
            box.scrollTop = box.scrollHeight;
            requestAnimationFrame(() => {
              requestAnimationFrame(() => {
                done(box.scrollHeight - box.clientHeight - box.scrollTop);
              });
            });
          }),
      ),
    )
    .toBeLessThanOrEqual(1);
}

/** Where the last row drawn in the box of the rows ends and where the bar of the ticked issues begins, from the top of the window. */
export async function lastRowEndAndBarTop(
  scroll: Locator,
  bar: Locator,
): Promise<{ readonly rowEnd: number; readonly barTop: number }> {
  const [rowEnd, barTop] = await Promise.all([
    scroll
      .locator("[role='row']")
      .last()
      .evaluate((row) => row.getBoundingClientRect().bottom),
    bar.evaluate((element) => element.getBoundingClientRect().top),
  ]);
  return { rowEnd, barTop };
}

/**
 * Scrolls a box of rows of a list to its end and says how much room is under its last row. The
 * rows are read in one go in the page: the list draws only the rows in sight, so a row found in one
 * step can be gone in the next while it draws the end; no row (yet) is no room.
 */
export function roomUnderLastRow(scroll: Locator): Promise<number> {
  return scroll.evaluate((box, row) => {
    box.scrollTop = box.scrollHeight;
    const rows = box.querySelectorAll(`[data-part="${row}"]`);
    const last = rows.item(rows.length - 1);
    return last === null
      ? Number.NEGATIVE_INFINITY
      : box.getBoundingClientRect().bottom - last.getBoundingClientRect().bottom;
  }, part.listRow);
}
