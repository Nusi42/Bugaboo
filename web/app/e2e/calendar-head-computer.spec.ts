import { ada } from "./support/accounts.ts";
import { openCalendar, prepareCalendar } from "./support/calendar.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";

// The month of the calendar on a computer (PLAN-09, UX §4.16, §6.5): its days scroll in their box, and the row of
// the weekdays stays at the top of the box, so the last weeks still have the names of their columns over them.

test.use({ preparation: { run: prepareCalendar } });

test("plan_09_the_weekdays_stay_over_the_month_when_it_scrolls_to_its_last_week", async ({
  page,
}) => {
  await signIn(page, ada);
  await openCalendar(page);
  const weekdays = partIn(page, "calendar-weekday");
  await expect(weekdays).toHaveCount(7);
  await partIn(page, "calendar-day").last().scrollIntoViewIfNeeded();
  // The first week is out of view; the weekdays are not.
  await expect(partIn(page, "calendar-day").first()).not.toBeInViewport();
  await expect(weekdays.first()).toBeInViewport({ ratio: 1 });
  await expect(weekdays.last()).toBeInViewport({ ratio: 1 });
});
