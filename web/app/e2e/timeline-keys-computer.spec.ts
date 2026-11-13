import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  dayFromToday,
  dragAcross,
  movedLater,
  openTimeline,
  planned,
  prepareTimeline,
  pressOf,
} from "./support/timeline.ts";
import { datesAs } from "./support/timeline-api.ts";

// The keys of a bar on a computer (PLAN-03, the maintainer's answers M8-F83 and M8-F84): the
// presses of a run are one change with one Undo, and moving an end keeps the day the bar showed
// there, which the sentence of the change names.

test.use({ preparation: { run: prepareTimeline } });

test("plan_03_presses_of_a_key_in_a_row_are_one_change_with_one_undo", async ({ page, server }) => {
  await signIn(page, ada);
  await openTimeline(page);
  await pressOf(page, planned.pressed).focus();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await expect
    .poll(() => datesAs(server, ada, planned.pressed))
    .toEqual({ start: dayFromToday(7), due: dayFromToday(9) });
  const said = movedLater(planned.pressed, t("timeline.amount-days", { count: 3 }));
  await expect(page.getByText(said)).toHaveCount(1);
  const undo = page.getByRole("button", { name: t("common.undo"), exact: true });
  await expect(undo).toHaveCount(1);
  await undo.click();
  await expect
    .poll(() => datesAs(server, ada, planned.pressed))
    .toEqual({ start: dayFromToday(4), due: dayFromToday(6) });
});

test("plan_03_moving_an_end_keeps_the_day_the_bar_showed_and_says_both_dates", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openTimeline(page);
  // An issue with a start date only is a bar of one day; Alt and an arrow move its end.
  await pressOf(page, planned.oneDate).focus();
  await page.keyboard.press("Alt+ArrowRight");
  await expect
    .poll(() => datesAs(server, ada, planned.oneDate))
    .toEqual({ start: dayFromToday(5), due: dayFromToday(6) });
  const said = t("timeline.dates-changed", { key: planned.oneDate, dates: "" });
  await expect(page.getByText(said.trim())).toHaveCount(1);
});

/** The width of one day at the zoom of weeks, as the theme file draws it (`size-day-week`). */
const weekDayPixels = 16;

test("plan_03_a_drag_during_a_run_of_keys_saves_both_in_order", async ({ page, server }) => {
  await signIn(page, ada);
  await openTimeline(page);
  const bar = pressOf(page, planned.keyThenDrag);
  await bar.focus();
  await page.keyboard.press("ArrowRight");
  // The drag comes before the keys rested: the run is saved first, then the drag on top of it.
  await dragAcross(page, bar, weekDayPixels * 7);
  await expect
    .poll(() => datesAs(server, ada, planned.keyThenDrag))
    .toEqual({ start: dayFromToday(9), due: dayFromToday(10) });
  // One Undo, the drag's: it puts back the dates the run of keys saved.
  const undo = page.getByRole("button", { name: t("common.undo"), exact: true });
  await expect(undo).toHaveCount(1);
  await undo.click();
  await expect
    .poll(() => datesAs(server, ada, planned.keyThenDrag))
    .toEqual({ start: dayFromToday(2), due: dayFromToday(3) });
});

test("plan_03_an_undo_is_withdrawn_when_a_newer_change_of_its_issue_starts", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openTimeline(page);
  const bar = pressOf(page, planned.twice);
  const first = movedLater(planned.twice, t("timeline.amount-days", { count: 3 }));
  await dragAcross(page, bar, weekDayPixels * 3);
  await expect(page.getByText(first)).toHaveCount(1);
  // A second change of the same issue: the Undo of the first would put back older dates.
  await dragAcross(page, bar, weekDayPixels * 3);
  await expect(page.getByText(first)).toHaveCount(1);
  const undo = page.getByRole("button", { name: t("common.undo"), exact: true });
  await expect(undo).toHaveCount(1);
  await expect
    .poll(() => datesAs(server, ada, planned.twice))
    .toEqual({ start: dayFromToday(8), due: dayFromToday(9) });
  await undo.click();
  await expect
    .poll(() => datesAs(server, ada, planned.twice))
    .toEqual({ start: dayFromToday(5), due: dayFromToday(6) });
});
