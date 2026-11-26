import { t } from "@bugaboo/messages";

import { ada, signInThroughApi } from "./support/accounts.ts";
import { itemOf } from "./support/calendar-places.ts";
import { prepareCalendar } from "./support/calendar.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue } from "./support/issues.ts";
import { dayFromToday } from "./support/person-day.ts";
import { changeIssue } from "./support/timeline.ts";

// The calendar on a phone steps through time as on a computer (PLAN-09, UX §4.16): *Earlier* and *Later* move the
// list of the days, and *Today* brings it back, so an issue that is overdue and one due in seven weeks are reached.

test.use({ preparation: { run: prepareCalendar } });

/** How far back the overdue issue is due, and how far ahead the later one: outside the days the phone lists first. */
const OVERDUE_DAYS = -10;
const LATER_DAYS = 49;

test("plan_09_the_phone_reaches_an_overdue_issue_and_one_due_in_seven_weeks", async ({
  page,
  server,
}) => {
  const admin = await signInThroughApi(server, ada);
  const overdue = await createIssue(admin, "WEB", "Overdue on the phone");
  const later = await createIssue(admin, "WEB", "Due in seven weeks");
  await changeIssue(admin, overdue.key, { due_date: dayFromToday(OVERDUE_DAYS) });
  await changeIssue(admin, later.key, { due_date: dayFromToday(LATER_DAYS) });
  await admin.dispose();
  await signIn(page, ada);
  await page.goto("/p/WEB/issues?show=calendar");
  const earlier = page.getByRole("button", { name: t("calendar.earlier"), exact: true });
  const next = page.getByRole("button", { name: t("calendar.later"), exact: true });
  const today = page.getByRole("button", { name: t("calendar.today"), exact: true });
  await expect(earlier).toBeVisible();
  await expect(itemOf(page, overdue.key)).toHaveCount(0);
  await expect(itemOf(page, later.key)).toHaveCount(0);
  await earlier.click();
  await expect(itemOf(page, overdue.key)).toBeVisible();
  await today.click();
  await expect(itemOf(page, overdue.key)).toHaveCount(0);
  await next.click();
  await expect(itemOf(page, later.key)).toBeVisible();
});
