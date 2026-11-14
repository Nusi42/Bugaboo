import { type Locator } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";
import { sampleAdministrator, samplePeople } from "./support/sample.ts";
import { paths } from "../src/shared/paths.ts";

// A virtualized list in a page scrolls with its pane (UX §6.5): the list of people has no scroll box
// of its own, the main column scrolls it with the cards under it, and the list draws the rows that
// come into the window.

test.use({ sample: true });

/** The main column that scrolls the list. */
const mainColumn = "[data-part~='app-layout.main']";

/** Whether the focused row of the list stands whole inside the main column. */
function focusedRowInColumn(list: Locator): Promise<boolean> {
  return list.evaluate((box, column) => {
    const row = box.querySelector("[role='option']:focus, [role='option'][data-focused]");
    const view = box.closest(column);
    if (row === null || view === null) {
      return false;
    }
    const [place, room] = [row.getBoundingClientRect(), view.getBoundingClientRect()];
    return place.top >= room.top - 1 && place.bottom <= room.bottom + 1;
  }, mainColumn);
}

/** Scrolls the main column so the middle of the list stands in its middle, and counts the rows in view. */
function rowsInViewOfTheMiddle(list: Locator): Promise<number> {
  return list.evaluate((box, selector) => {
    const column = box.closest(selector);
    if (column === null) {
      return 0;
    }
    const [rows, view] = [box.getBoundingClientRect(), column.getBoundingClientRect()];
    column.scrollTop += (rows.top + rows.bottom) / 2 - (view.top + view.bottom) / 2;
    let seen = 0;
    for (const row of box.querySelectorAll("[role='option']")) {
      const place = row.getBoundingClientRect();
      seen += place.bottom > view.top && place.top < view.bottom ? 1 : 0;
    }
    return seen;
  }, mainColumn);
}

test("ux_06 the list of people scrolls with its page and draws the rows that come into view", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.people);
  const list = page.getByRole("listbox", { name: t("people.list"), exact: true });
  await expect(list.getByRole("option").first()).toBeVisible();
  // As high as its rows: nothing in it scrolls.
  expect(await list.evaluate((box) => box.scrollHeight - box.clientHeight)).toBeLessThanOrEqual(1);
  const first = await list.getByRole("option").first().textContent();
  // The main column brings the middle of the list into view (again while the rows drawn change the
  // height the list guessed), and the rows there are drawn.
  await expect.poll(() => rowsInViewOfTheMiddle(list)).toBeGreaterThan(0);
  await expect(
    list
      .getByRole("option")
      .filter({ hasText: String(first) })
      .first(),
  ).not.toBeInViewport();
  // Only the rows near the window are drawn, far fewer than the people of the sample.
  const people = samplePeople().length;
  expect(await list.getByRole("option").count()).toBeLessThan(people / 2);
});

test("ux_06 the arrow keys bring the focused person into view in the main column", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  await page.goto(paths.people);
  const list = page.getByRole("listbox", { name: t("people.list"), exact: true });
  await expect(list.getByRole("option").first()).toBeVisible();
  await list.getByRole("option").first().focus();
  // Further down than the rows drawn at the start: the column scrolls with the focus.
  const steps = (await list.getByRole("option").count()) + 5;
  await inOrder(Array.from({ length: steps }, () => () => page.keyboard.press("ArrowDown")));
  await expect.poll(() => focusedRowInColumn(list)).toBe(true);
  // The main column scrolled to it; the list itself did not.
  expect(await list.evaluate((box) => box.scrollTop)).toBe(0);
  expect(await page.locator(mainColumn).evaluate((column) => column.scrollTop)).toBeGreaterThan(0);
});
