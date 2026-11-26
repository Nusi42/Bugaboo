import { t } from "@bugaboo/messages";

import { dayInSentence, monthTitle } from "../src/shared/clock.ts";
import { dayText } from "../src/shared/format.ts";
import { ada } from "./support/accounts.ts";
import {
  calendarOf,
  dayOf,
  dragToDay,
  itemOf,
  movedTo,
  pressOf,
  weekDays,
} from "./support/calendar-places.ts";
import { openCalendar, openProjectCalendar, prepareCalendar, scene } from "./support/calendar.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { expectStill, landmarkOf, movementWhile, partLandmark } from "./support/movement.ts";
import { datesAs } from "./support/timeline-api.ts";

// The calendar on a computer (PLAN-09, UX §4.16, M10-Q5): a search shown as a month or a week by its due
// date, an issue dragged to another day with *Undo*, and *Change due date …* with the keys only.

test.use({ preparation: { run: prepareCalendar } });

test("plan_09_a_saved_search_shows_as_a_month_and_a_week_calendar_by_due_date", async ({
  page,
}) => {
  const { today, crowded } = weekDays();
  await signIn(page, ada);
  await openCalendar(page);
  // A month: its name, the weekdays above, today marked, and the crowded day with *+3 more*.
  await expect(page.getByRole("heading", { name: monthTitle(today) })).toBeVisible();
  await expect(page.getByLabel(t("timeline.show-as"))).toHaveValue("calendar");
  await expect(calendarOf(page).locator('[data-part="calendar-weekday"]')).toHaveCount(7);
  await expect(dayOf(page, today)).toHaveAttribute("aria-current", "date");
  const more = dayOf(page, crowded).getByRole("button", { name: /^\+\d+ more on /u });
  await more.click();
  // The whole list of the day opens, with every issue on it.
  const list = page.getByRole("dialog");
  await Promise.all(
    scene.crowd.map((key) => expect(list.getByText(key, { exact: true })).toBeVisible()),
  );
  await page.keyboard.press("Escape");
  // A week: seven days, each issue of the crowded day on it.
  await page
    .getByRole("group", { name: t("calendar.span") })
    .getByRole("radio", { name: t("calendar.span-week") })
    .check();
  await expect(calendarOf(page).locator('[data-part="calendar-day"]')).toHaveCount(7);
  await Promise.all(scene.crowd.map((key) => expect(itemOf(page, key)).toBeVisible()));
});

test("plan_09_dragging_an_issue_to_another_day_changes_its_due_date_and_undo_takes_it_back", async ({
  page,
  server,
}) => {
  const { today, other } = weekDays();
  await signIn(page, ada);
  await openProjectCalendar(page);
  const grid = await landmarkOf(calendarOf(page), "grid");
  const kept = await landmarkOf(itemOf(page, scene.crowdFirst), "an issue of another day");
  const tools = partLandmark("tools", "calendar-tools");
  // A drop moves only the issue dropped: the tools, the grid and the other issues stand still.
  const moving = await movementWhile(page, [grid, kept, tools], async () => {
    await dragToDay(page, scene.dragged, dayOf(page, other));
    await expect(page.getByText(movedTo(scene.dragged, dayInSentence(other)))).toBeVisible();
  });
  expectStill(moving, {});
  await expect.poll(async () => (await datesAs(server, ada, scene.dragged)).due).toBe(other);
  await expect(dayOf(page, other).getByText(scene.dragged, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect.poll(async () => (await datesAs(server, ada, scene.dragged)).due).toBe(today);
});

test("plan_09_change_due_date_does_what_a_drag_does", async ({ page, server }) => {
  const { today, other } = weekDays();
  await signIn(page, ada);
  await openProjectCalendar(page);
  // Keyboard only: the issue, its menu, the entry of the menu, the box for the day.
  await pressOf(page, scene.typed).focus();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: t("calendar.change-due-date") }).focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", {
    name: t("calendar.change-title", { key: scene.typed, field: t("calendar.due-date") }),
  });
  await expect(window).toBeVisible();
  await window.getByLabel(t("issue-field.due-date")).fill(dayText(other));
  await page.keyboard.press("Enter");
  await expect(page.getByText(movedTo(scene.typed, dayInSentence(other)))).toBeVisible();
  await expect.poll(async () => (await datesAs(server, ada, scene.typed)).due).toBe(other);
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect.poll(async () => (await datesAs(server, ada, scene.typed)).due).toBe(today);
});
