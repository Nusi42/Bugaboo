import { t } from "@bugaboo/messages";

import { ana } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";
import {
  arrowOf,
  barOf,
  chart,
  dayFromToday,
  dragAcross,
  goalTitle,
  movedLater,
  openPlan,
  openTimeline,
  planned,
  pressOf,
  prepareTimeline,
  trayHandle,
} from "./support/timeline.ts";
import {
  createOwnDates,
  datesAs,
  idsOf,
  ownValueOf,
  relationsOf,
  setOwnValues,
} from "./support/timeline-api.ts";

// The timeline on a computer (PLAN-01 … PLAN-04, UX §4.7, ADR-0043): a saved search shown as a
// timeline, its bars, arrows, goals and tray, dragged with Undo and moved by keys and typed ways.

test.use({ preparation: { run: prepareTimeline } });

/** The width of one day at the zoom of weeks, as the theme file draws it (`size-day-week`). */
const weekDayPixels = 16;

/** The head of the group of the goal. */
function goalHead(page: Parameters<typeof chart>[0]) {
  return chart(page).locator('[data-part="timeline-row.toggle"]').filter({ hasText: goalHeadName });
}

/** The name of the head of the group of the goal: its Issue ID and its title. */
const goalHeadName = new RegExp(`^${planned.goal}\\s?${goalTitle}`, "u");

test("plan_01_a_saved_search_shows_as_a_timeline_with_its_dates", async ({ page }) => {
  await signIn(page, ada);
  await openPlan(page);
  // Grouped by goal: the goal heads its two issues, each bar on its days.
  await expect(goalHead(page)).toBeVisible();
  const bar = pressOf(page, planned.first);
  await expect(bar).toBeVisible();
  await expect(bar).toHaveAccessibleName(new RegExp(`^${planned.first} `, "u"));
  // The view keeps its kind: *Show as* says Timeline.
  await expect(page.getByLabel(t("timeline.show-as"))).toHaveValue("timeline");
});

test("plan_01_a_view_with_own_date_fields_draws_and_drags_those_fields", async ({
  page,
  server,
}) => {
  const fields = await createOwnDates(server);
  await setOwnValues(server, planned.ownDates, {
    [fields.begins]: dayFromToday(2),
    [fields.ends]: dayFromToday(4),
  });
  await signIn(page, ada);
  await openTimeline(page, `&dates=${encodeURIComponent("begins,ends")}`);
  const bar = pressOf(page, planned.ownDates);
  await expect(bar).toBeVisible();
  // A key moves the bar a day later: the own fields change, Start date and Due date stay empty.
  await bar.focus();
  await page.keyboard.press("ArrowRight");
  const said = movedLater(planned.ownDates, t("timeline.amount-days", { count: 1 }));
  await expect(page.getByText(said)).toBeVisible();
  await expect
    .poll(() => ownValueOf(server, planned.ownDates, fields.begins))
    .toBe(dayFromToday(3));
  expect(await datesAs(server, ada, planned.ownDates)).toEqual({ start: null, due: null });
});

test("plan_02_an_arrow_joins_an_issue_that_must_finish_first_to_the_one_that_waits", async ({
  page,
  server,
}) => {
  const [first, waits] = await idsOf(server, [planned.first, planned.waits]);
  await signIn(page, ada);
  await openPlan(page);
  await expect(arrowOf(page, [first, waits])).toHaveCount(1);
  // There is no arrow the other way.
  await expect(arrowOf(page, [waits, first])).toHaveCount(0);
});

test("plan_02_dragging_the_dot_makes_a_dependency_and_add_dependency_does_the_same", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openTimeline(page);
  const from = barOf(page, planned.dotFrom);
  await pressOf(page, planned.dotFrom).click();
  await from.locator('[data-part="timeline-bar.dot"]').dragTo(barOf(page, planned.dotTo));
  await expect(
    page.getByText(
      t("timeline.dependency-made", { first: planned.dotFrom, waiting: planned.dotTo }),
    ),
  ).toBeVisible();
  // *Add dependency…* on the card: the other issue is found with the issue finder.
  await pressOf(page, planned.found).click();
  await page.getByRole("button", { name: t("timeline.add-dependency"), exact: true }).click();
  const window = page.getByRole("dialog", {
    name: t("timeline.add-dependency-title", { key: planned.found }),
  });
  await window.getByRole("radio", { name: t("timeline.waits", { key: planned.found }) }).check();
  await window.getByLabel(t("search.bar.issue-search")).fill(planned.dotTo);
  await window
    .getByRole("button", { name: t("search.bar.issue-take", { issue: planned.dotTo }) })
    .click();
  await window.getByRole("button", { name: t("timeline.add-it") }).click();
  await expect(
    page.getByText(t("timeline.dependency-made", { first: planned.dotTo, waiting: planned.found })),
  ).toBeVisible();
  const relations = await relationsOf(server, planned.dotTo);
  expect(relations).toContain(planned.found);
  expect(relations).toContain(planned.dotFrom);
});

test("plan_03_dragging_a_bar_moves_its_dates_and_undo_takes_it_back", async ({ page, server }) => {
  const before = await datesAs(server, ada, planned.dragged);
  await signIn(page, ada);
  await openTimeline(page);
  await dragAcross(page, pressOf(page, planned.dragged), weekDayPixels * 7);
  const said = movedLater(planned.dragged, t("timeline.amount-weeks", { count: 1 }));
  await expect(page.getByText(said)).toBeVisible();
  await expect
    .poll(async () => (await datesAs(server, ada, planned.dragged)).start)
    .toBe(dayFromToday(9));
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect.poll(() => datesAs(server, ada, planned.dragged)).toEqual(before);
});

test("plan_03_an_issue_the_person_may_not_change_cannot_be_dragged_and_says_why", async ({
  page,
  server,
}) => {
  const before = await datesAs(server, ada, planned.keys);
  // Ana is a Viewer of WEB: she sees the bars and may change none.
  await signIn(page, ana);
  await openTimeline(page);
  const bar = pressOf(page, planned.keys);
  const why = t("timeline.locked", { key: planned.keys });
  await expect(bar).toHaveAttribute("title", why);
  // A drag starts nothing: the bar does not move.
  await dragAcross(page, bar, weekDayPixels * 3);
  await expect(barOf(page, planned.keys)).not.toHaveAttribute("data-state", /dragging/u);
  // A key that would move it says why it does not.
  await bar.focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByText(why).first()).toBeVisible();
  expect(await datesAs(server, ada, planned.keys)).toEqual(before);
});

test("plan_03_set_dates_does_what_a_drag_does", async ({ page, server }) => {
  await signIn(page, ada);
  await openTimeline(page);
  // Keyboard only: the bar takes the focus, Enter selects it, the card has *Set dates…*.
  await pressOf(page, planned.keys).focus();
  await page.keyboard.press("Enter");
  const card = page.getByRole("region", { name: t("timeline.card", { key: planned.keys }) });
  await card.getByRole("button", { name: t("timeline.set-dates"), exact: true }).focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", {
    name: t("timeline.set-dates-title", { key: planned.keys }),
  });
  await expect(window).toBeVisible();
  await window.getByLabel(t("issue-field.start-date")).focus();
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type(dayFromToday(10));
  await page.keyboard.press("Tab");
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type(dayFromToday(11));
  await page.keyboard.press("Enter");
  await expect
    .poll(() => datesAs(server, ada, planned.keys))
    .toEqual({ start: dayFromToday(10), due: dayFromToday(11) });
  // An arrow key moves the selected bar a day, with Shift a week.
  await pressOf(page, planned.keys).focus();
  await page.keyboard.press("Shift+ArrowLeft");
  await expect
    .poll(() => datesAs(server, ada, planned.keys))
    .toEqual({ start: dayFromToday(3), due: dayFromToday(4) });
});

test("plan_04_rows_group_fold_and_zoom_from_day_to_quarter", async ({ page }) => {
  await signIn(page, ada);
  await openPlan(page);
  const head = goalHead(page);
  await expect(head).toHaveAttribute("aria-expanded", "true");
  await expect(barOf(page, planned.first)).toBeVisible();
  await head.click();
  await expect(head).toHaveAttribute("aria-expanded", "false");
  await expect(barOf(page, planned.first)).toHaveCount(0);
  await head.click();
  const zoom = page.getByRole("group", { name: t("timeline.zoom") });
  await inOrder(
    [
      t("timeline.zoom-day"),
      t("timeline.zoom-week"),
      t("timeline.zoom-month"),
      t("timeline.zoom-quarter"),
    ].map((name) => async () => {
      await zoom.getByRole("radio", { name }).check();
      await expect(barOf(page, planned.first)).toBeVisible();
    }),
  );
  await expect(page.locator('[data-part="day-scale.cell"]').first()).toContainText("Q");
  await expect(page).toHaveURL(/zoom=quarter/u);
});

test("plan_04_a_goal_with_a_due_date_is_a_diamond", async ({ page }) => {
  await signIn(page, ada);
  await openPlan(page);
  const goalRow = page.locator('[data-part="timeline-row"]').filter({
    has: page.locator('[data-part="timeline-row.toggle"]').filter({ hasText: goalHeadName }),
  });
  const diamond = goalRow.locator('[data-part="timeline-diamond"]');
  await expect(diamond).toHaveCount(1);
  await expect(goalRow.locator('[data-part="timeline-bracket"]')).toHaveCount(1);
  // Screen readers hear the goal from the head of its group: its days and its due date.
  await expect(goalHead(page)).toHaveAccessibleName(
    new RegExp(t("timeline.goal-due", { key: planned.goal, day: ".+" }), "u"),
  );
});

test("plan_01_an_issue_from_not_scheduled_yet_gets_dates_by_drag_or_by_set_dates", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openTimeline(page);
  const tray = page.getByRole("region", { name: t("timeline.not-scheduled") });
  await expect(tray).toContainText(planned.undated);
  // Dragged onto the chart, it starts on the day it was let go.
  await trayHandle(tray, planned.undated).dragTo(chart(page), {
    targetPosition: { x: 400, y: 60 },
  });
  await expect.poll(async () => (await datesAs(server, ada, planned.undated)).start).not.toBeNull();
  // *Set dates…* does the same from the keyboard.
  const item = tray.getByRole("listitem").filter({ hasText: planned.trayKeys });
  await item.getByRole("button", { name: t("timeline.set-dates") }).click();
  const window = page.getByRole("dialog", {
    name: t("timeline.set-dates-title", { key: planned.trayKeys }),
  });
  await window.getByLabel(t("issue-field.start-date")).fill(dayFromToday(6));
  await window.getByLabel(t("issue-field.due-date")).fill(dayFromToday(7));
  await window.getByRole("button", { name: t("timeline.save-dates") }).click();
  await expect
    .poll(() => datesAs(server, ada, planned.trayKeys))
    .toEqual({ start: dayFromToday(6), due: dayFromToday(7) });
  await expect(barOf(page, planned.trayKeys)).toBeVisible();
});
