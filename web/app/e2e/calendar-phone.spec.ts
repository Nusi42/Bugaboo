import { t } from "@bugaboo/messages";

import { weekdayAndDate } from "../src/shared/clock.ts";
import { dayText } from "../src/shared/format.ts";
import { regionOf } from "../src/shared/region.ts";
import { ada } from "./support/accounts.ts";
import { chooseChangeDay, itemOf, weekDays } from "./support/calendar-places.ts";
import { prepareCalendar, releaseName, scene } from "./support/calendar.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { datesAs } from "./support/timeline-api.ts";

// The calendar on a phone (PLAN-09, UX §4.16): a list of the days to come, the days without issues left
// out, *Change due date …* on each issue, and nothing dragged.

test.use({ preparation: { run: prepareCalendar } });

test("plan_09_the_phone_shows_a_day_by_day_list", async ({ page, server }) => {
  const { today, other } = weekDays();
  await signIn(page, ada);
  await page.goto("/p/WEB/issues?show=calendar");
  // Today is a heading with its issues; a day without any is not there.
  const day = page.getByRole("heading", { name: weekdayAndDate(today, regionOf()) });
  await expect(day).toBeVisible();
  await expect(page.getByRole("heading", { name: weekdayAndDate(other, regionOf()) })).toHaveCount(
    0,
  );
  await expect(itemOf(page, scene.dragged)).toBeVisible();
  // The release stands on its release date: today, the day the scene gives it.
  await expect(
    day
      .locator("xpath=ancestor::section[1]")
      .getByRole("link", { name: t("calendar.release-on", { name: releaseName }) }),
  ).toBeVisible();
  // Nothing is dragged on a phone: *Change due date …* does the same.
  await expect(page.locator('[draggable="true"]')).toHaveCount(0);
  await chooseChangeDay(page, scene.dragged);
  const window = page.getByRole("dialog", {
    name: t("calendar.change-title", { key: scene.dragged, field: t("calendar.due-date") }),
  });
  await window.getByLabel(t("issue-field.due-date")).fill(dayText(other));
  await window.getByRole("button", { name: t("calendar.save-day") }).click();
  await expect.poll(async () => (await datesAs(server, ada, scene.dragged)).due).toBe(other);
});
