import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { weekdayName } from "../../src/shared/clock.ts";
import { addDays } from "../../src/shared/format.ts";

/** The name of a day as its section and its cells say it. */
function dayName(first: string, column: number): string {
  return weekdayName(addDays(first, column), "long");
}

/** The tabs of the days of the week. */
export function dayTabs(page: Page): Locator {
  return page.getByRole("tablist", { name: t("time.sheet.days") }).getByRole("tab");
}

/** Shows the day of the `column`th tab (Monday is 0): one day at a time is what a phone has. */
export async function showDay(page: Page, column: number): Promise<void> {
  await dayTabs(page).nth(column).click();
}

/** The section of a day below the tabs. */
export function dayOnPhone(page: Page, first: string, column: number): Locator {
  return page.getByRole("region", { name: dayName(first, column), exact: true });
}

/** The box of a row (`Start`, an issue ID) on the day that is shown. */
export function phoneCell(page: Page, first: string, name: string, column: number): Locator {
  const label = new RegExp(`^${name}, ${dayName(first, column)}`, "u");
  return dayOnPhone(page, first, column).getByRole("textbox", { name: label });
}

/** The figure beside a name (*Working hours*, *Booked on issues*) in the day that is shown. */
export function phoneFigure(page: Page, first: string, name: string, column: number): Locator {
  return dayOnPhone(page, first, column)
    .locator('[data-part="sheet.line"]')
    .filter({ has: page.getByText(name, { exact: true }) })
    .locator('[data-part="sheet.figure"]');
}

/** The figure of the week on top of the days. */
export function weekFigure(page: Page, name: string): Locator {
  return page
    .locator('[data-part="sheet.phone"] > [data-part="sheet.block"]')
    .first()
    .locator('[data-part="sheet.line"]')
    .filter({ has: page.getByText(name, { exact: true }) })
    .locator('[data-part="sheet.figure"]');
}
