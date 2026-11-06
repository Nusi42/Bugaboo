import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { addDays } from "../../src/shared/format.ts";
import { signInThroughApi } from "./accounts.ts";
import { ana, days, good } from "./demo-m6-common.ts";
import { signIn } from "./flows.ts";
import { layoutOf } from "./layout.ts";
import { type Instance } from "./server.ts";
import { phoneCell } from "./timesheet-phone.ts";
import {
  cellOf,
  chooseUtc,
  columnOf,
  giveReason,
  historyButtons,
  typeInto,
  openDay,
  openWeek,
  weekOf,
  workedOn,
} from "./timesheet.ts";

// The steps of the exit demo of M6 that are about the timesheet (`m6-exit-demo.spec.ts`): the
// warning of a long day, and a day changed afterwards with a reason.

/** How far back the long day is, in days: older than the weeks the sample recorded. */
const LONG_DAY_BACK = 120;

/** Step 3: the timesheet shows the warning of a long day. */
export async function aLongDayShowsItsWarning(page: Page, server: Instance): Promise<void> {
  const session = await signInThroughApi(server, ana());
  await chooseUtc(session);
  const long = weekOf(addDays(days().today, -LONG_DAY_BACK));
  await workedOn(session, long, "06:00", "19:00");
  await session.dispose();
  await signIn(page, ana());
  await openWeek(page, long);
  await expect(page.getByText(/hours? .*worked/u).first()).toBeVisible();
  await good(page, "03-the-warning-of-a-long-day");
}

/** Step 4: Ana changes the end of yesterday and gives her reason; the day shows *Changed*. */
export async function anaChangesYesterdayWithAReason(page: Page, server: Instance): Promise<void> {
  const { yesterday } = days();
  const session = await signInThroughApi(server, ana());
  await chooseUtc(session);
  await workedOn(session, yesterday, "08:00", "17:00");
  await session.dispose();
  await signIn(page, ana());
  const first = weekOf(yesterday);
  const column = columnOf(yesterday);
  const phone = layoutOf(page) === "phone";
  await openDay(page, first, column, false);
  const end = phone
    ? phoneCell(page, first, t("time.sheet.end"), column)
    : cellOf(page, t("time.sheet.end"), column);
  await typeInto(end, "18:35");
  await expect(page.getByRole("dialog")).toBeVisible();
  await good(page, "04-a-why-this-day-is-changed");
  await giveReason(page, "Left later than I noted");
  // The cell is left: no focus ring and no selected text in the picture.
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });
  await good(page, "04-b-a-day-changed-with-a-reason");
  await historyButtons(page).first().click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await good(page, "04-c-its-history");
}
