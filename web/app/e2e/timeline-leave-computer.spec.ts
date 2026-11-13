import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  chart,
  dayFromToday,
  openTimeline,
  planned,
  prepareTimeline,
  pressOf,
} from "./support/timeline.ts";
import { datesAs } from "./support/timeline-api.ts";

// A run of keys that has not rested yet is saved when the person leaves the timeline, inside the
// app or by reloading the page; and an Undo of an Undo brings the change back (PLAN-03).

test.use({ preparation: { run: prepareTimeline } });

test("plan_03_an_undo_of_an_undo_brings_the_change_back", async ({ page, server }) => {
  await signIn(page, ada);
  await openTimeline(page);
  await pressOf(page, planned.pressed).focus();
  await page.keyboard.press("ArrowRight");
  const moved = { start: dayFromToday(5), due: dayFromToday(7) };
  await expect.poll(() => datesAs(server, ada, planned.pressed)).toEqual(moved);
  const undo = page.getByRole("button", { name: t("common.undo"), exact: true });
  await undo.click();
  await expect
    .poll(() => datesAs(server, ada, planned.pressed))
    .toEqual({ start: dayFromToday(4), due: dayFromToday(6) });
  await expect(undo).toHaveCount(1);
  await undo.click();
  await expect.poll(() => datesAs(server, ada, planned.pressed)).toEqual(moved);
});

test("plan_03_a_run_of_keys_is_saved_when_the_timeline_is_left_in_the_app", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  // The list first, then the timeline in the app, so going back leaves it inside the app.
  await page.goto(`/search?s=${encodeURIComponent("project is WEB")}`);
  await page.getByLabel(t("timeline.show-as")).selectOption("timeline");
  await expect(chart(page)).toBeVisible();
  await pressOf(page, planned.keyThenDrag).focus();
  await page.keyboard.press("ArrowRight");
  // Going back moves no focus: only the timeline going away saves the run.
  await page.goBack();
  await expect(chart(page)).toHaveCount(0);
  await expect
    .poll(() => datesAs(server, ada, planned.keyThenDrag))
    .toEqual({ start: dayFromToday(2), due: dayFromToday(3) });
});

test("plan_03_a_run_of_keys_is_saved_when_the_page_is_reloaded", async ({ page, server }) => {
  await signIn(page, ada);
  await openTimeline(page);
  await pressOf(page, planned.twice).focus();
  await page.keyboard.press("ArrowRight");
  await page.reload();
  await expect
    .poll(() => datesAs(server, ada, planned.twice))
    .toEqual({ start: dayFromToday(3), due: dayFromToday(4) });
});
