import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { weekDaysOf, weekdayAndDate, weekFirstOf } from "../../src/shared/clock.ts";
import { regionOf } from "../../src/shared/region.ts";
import { letGoOn, takeUp } from "./hand-drag.ts";
import { addDays, dayFromToday, daysBetween } from "./person-day.ts";

// Where the parts of the calendar are on the page, and the days of the scene (PLAN-09, UX §4.16).

/**
 * Today, the day with the most issues and a day for a drop, all in the week of today by the first day of the
 * week of the person: the week is on screen whatever the span, and none stands outside the days of the grid.
 */
export function weekDays(): {
  readonly today: string;
  readonly crowded: string;
  readonly other: string;
  /** The day next to today inside the same week, and the arrow key that carries an issue to it. */
  readonly next: string;
  readonly arrow: string;
} {
  const today = dayFromToday(0);
  const days = weekDaysOf(weekFirstOf(today, regionOf().firstDay));
  const [crowded = today, other = today] = days.filter((day) => day !== today);
  const last = days.at(-1) === today;
  return {
    today,
    crowded,
    other,
    next: addDays(today, last ? -1 : 1),
    arrow: last ? "ArrowLeft" : "ArrowRight",
  };
}

/** The calendar of the page. */
export function calendarOf(page: Page): Locator {
  return page.locator('[data-part="calendar-grid"]');
}

/** The cell of a day, by the name the page gives it. */
export function dayOf(page: Page, day: string): Locator {
  return page.getByRole("article", { name: weekdayAndDate(day, regionOf()) });
}

/** The issue of the calendar with this readable ID: the item that holds it, which is dragged. */
export function itemOf(page: Page, key: string): Locator {
  return page
    .locator('[data-part="calendar-item"]')
    .filter({ has: page.getByText(key, { exact: true }) });
}

/** The button that fills an issue of the calendar: what is pressed, and picked up with the spacebar. */
export function pressOf(page: Page, key: string): Locator {
  return itemOf(page, key).locator('[data-part="calendar-item.press"]');
}

/** What a move says, in words: "WEB-4 due date moved to Fri 16 Oct". */
export function movedTo(key: string, day: string, field = t("calendar.due-date")): string {
  return t("calendar.moved", { key, field, day });
}

/** The menu button of an issue: the shared compact button, named by the Issue ID. */
export function actionsOf(page: Page, key: string): Locator {
  return itemOf(page, key).getByRole("button", { name: t("common.actions-for", { name: key }) });
}

/** Opens *Change due date …* from the menu of an issue, with the pointer. */
export async function chooseChangeDay(page: Page, key: string): Promise<void> {
  await actionsOf(page, key).click();
  await page.getByRole("button", { name: t("calendar.change-due-date") }).click();
}

/** Drags an issue to a day with the moves of a hand: the browser drags as it does for a person, never `dragTo`. */
export async function dragToDay(page: Page, key: string, day: Locator): Promise<void> {
  // The grid scrolls inside its box: a day below the visible part is scrolled to first, as a person does, or the
  // pointer would let go over what is under the box.
  await day.scrollIntoViewIfNeeded();
  await takeUp(page, pressOf(page, key));
  await letGoOn(page, day);
}

/** The arrow keys that carry an issue from one day to another in the same grid: a day for each press. */
export function keysTo(from: string, to: string): readonly string[] {
  const days = daysBetween(from, to);
  return Array.from({ length: Math.abs(days) }, () => (days < 0 ? "ArrowLeft" : "ArrowRight"));
}
