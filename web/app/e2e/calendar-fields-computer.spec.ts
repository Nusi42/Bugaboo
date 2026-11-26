import { settingDefaults, settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { dayInSentence, weekFirstOf, weekdayName } from "../src/shared/clock.ts";
import { ana } from "./support/access.ts";
import { ada, signInThroughApi } from "./support/accounts.ts";
import {
  calendarOf,
  dayOf,
  dragToDay,
  itemOf,
  movedTo,
  pressOf,
  actionsOf,
  weekDays,
} from "./support/calendar-places.ts";
import {
  goalTitle,
  openProjectCalendar,
  ownFields,
  prepareCalendar,
  releaseName,
  scene,
} from "./support/calendar.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { datesAs, ownValueOf } from "./support/timeline-api.ts";

// The calendar on a computer (PLAN-09, UX §4.16, M10-Q5, M10-Q8): an issue picked up with the spacebar,
// *Dates from*, an issue that may not be moved, the marks of goals and releases, and the first day of the
// week of the person.

test.use({ preparation: { run: prepareCalendar } });

test("plan_09_a_picked_up_issue_is_carried_with_the_arrow_keys_and_put_down", async ({
  page,
  server,
}) => {
  const { arrow, next } = weekDays();
  await signIn(page, ada);
  await openProjectCalendar(page);
  const press = pressOf(page, scene.child);
  await press.focus();
  // The spacebar picks it up, an arrow key carries it a day, the spacebar puts it down.
  await page.keyboard.press("Space");
  await expect(itemOf(page, scene.child)).toHaveAttribute("data-state", /picked/u);
  await page.keyboard.press(arrow);
  await page.keyboard.press("Space");
  await expect.poll(async () => (await datesAs(server, ada, scene.child)).due).toBe(next);
  await expect(pressOf(page, scene.child)).toBeFocused();
});

test("plan_09_dates_from_a_start_date_or_an_own_date_field_draws_and_drags_that_field", async ({
  page,
  server,
}) => {
  const { other } = weekDays();
  await signIn(page, ada);
  await openProjectCalendar(page, "&span=week&from=start%20date");
  // *Dates from* says what the calendar stands on; only the issue with a start date is there.
  await expect(page.getByLabel(t("calendar.dates-from"))).toBeVisible();
  await expect(itemOf(page, scene.startOnly)).toBeVisible();
  await expect(itemOf(page, scene.dragged)).toHaveCount(0);
  await dragToDay(page, scene.startOnly, dayOf(page, other));
  await expect(
    page.getByText(movedTo(scene.startOnly, dayInSentence(other), t("calendar.start-date"))),
  ).toBeVisible();
  await expect
    .poll(() => datesAs(server, ada, scene.startOnly))
    .toEqual({ start: other, due: null });
  // An own date field: the issue that has one stands on it, and its due date stays empty.
  await openProjectCalendar(page, "&span=week&from=begins");
  await expect(itemOf(page, scene.ownOnly)).toBeVisible();
  await dragToDay(page, scene.ownOnly, dayOf(page, other));
  await expect.poll(() => ownValueOf(server, scene.ownOnly, ownFields.begins)).toBe(other);
  expect((await datesAs(server, ada, scene.ownOnly)).due).toBeNull();
});

test("plan_09_an_issue_the_person_may_not_change_cannot_be_dragged_and_says_why", async ({
  page,
  server,
}) => {
  const before = await datesAs(server, ada, scene.locked);
  // Ana is a Viewer of WEB: she sees the calendar and may change no date.
  await signIn(page, ana);
  await openProjectCalendar(page);
  const item = itemOf(page, scene.locked);
  await expect(item).toBeVisible();
  const why = t("calendar.locked", { key: scene.locked, field: t("calendar.due-date") });
  await expect(item).toHaveAttribute("title", why);
  // What the button of the issue carries on pointing at it: the reason, then the issue.
  expect(await pressOf(page, scene.locked).getAttribute("title")).toContain(why);
  await expect(item).not.toHaveAttribute("draggable", "true");
  await expect(actionsOf(page, scene.locked)).toHaveCount(0);
  await dragToDay(page, scene.locked, dayOf(page, weekDays().other));
  expect(await datesAs(server, ada, scene.locked)).toEqual(before);
});

test("plan_09_releases_and_goals_stand_on_their_dates_as_marks", async ({ page }) => {
  const day = dayOf(page, weekDays().today);
  await signIn(page, ada);
  await openProjectCalendar(page);
  await expect(
    day.getByRole("link", { name: t("calendar.release-on", { name: releaseName }) }),
  ).toBeVisible();
  await expect(
    day.getByRole("link", { name: t("calendar.goal-due", { key: scene.goal, title: goalTitle }) }),
  ).toBeVisible();
  // A mark is not dragged.
  await expect(day.locator('[data-part="release-mark"][draggable="true"]')).toHaveCount(0);
});

test("plan_09_the_week_starts_on_the_first_day_of_the_persons_region", async ({ page, server }) => {
  const admin = await signInThroughApi(server, ada);
  const set = (value: string) =>
    admin.client.PUT("/api/v1/me/settings/{key}", {
      params: { path: { key: settingKeys.regionFirstDay } },
      body: { value },
    });
  await set("sunday");
  try {
    await signIn(page, ada);
    await openProjectCalendar(page);
    const first = weekFirstOf(weekDays().today, "sunday");
    await expect(
      calendarOf(page).locator('[data-part="calendar-day"]').first(),
    ).toHaveAccessibleName(new RegExp(`^${weekdayName(first, "long")}`, "u"));
  } finally {
    await set(settingDefaults.regionFirstDay);
  }
  await admin.dispose();
});
