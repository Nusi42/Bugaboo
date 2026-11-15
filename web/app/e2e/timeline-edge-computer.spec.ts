import { t } from "@bugaboo/messages";

import { ada, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";
import {
  barOf,
  changeIssue,
  chart,
  cutByTheEdge,
  dayFromToday,
  openPlan,
  planned,
  prepareTimeline,
} from "./support/timeline.ts";

// The edge of the chart (UX §4.7): the label after a bar and the name of a unit of the scale show
// whole wherever the chart opens. A label that has no room after its bar nor before it stays in
// sight at the end of the days on screen, over its bar, and the chart opens at the start of a unit,
// so its first name is not cut by the names of the rows.

test.use({ preparation: { run: prepareTimeline } });

test("plan_01_no_label_and_no_name_of_the_scale_is_cut_by_the_edge_of_the_chart", async ({
  page,
  server,
}) => {
  // A long bar from tomorrow on: its label has no room after it on screen, nor before it.
  const admin = await signInThroughApi(server, ada);
  await changeIssue(admin, planned.found, {
    start_date: dayFromToday(1),
    due_date: dayFromToday(40),
  });
  await admin.dispose();
  await signIn(page, ada);
  await openPlan(page);
  await expect.poll(() => cutByTheEdge(page)).toEqual([]);
  const zoom = page.getByRole("group", { name: t("timeline.zoom") });
  await inOrder(
    (["timeline.zoom-day", "timeline.zoom-month", "timeline.zoom-quarter"] as const).map(
      (each) => async () => {
        await zoom.getByRole("radio", { name: t(each) }).check();
        await expect(chart(page)).toBeVisible();
        await expect.poll(() => cutByTheEdge(page)).toEqual([]);
      },
    ),
  );
});

test("plan_01_a_short_bar_near_the_end_of_the_days_on_screen_keeps_its_label_whole", async ({
  page,
  server,
}) => {
  // A bar of one day at each place from just after today to past the end of the days on screen:
  // where its label fits neither after it nor before it (the Today line is in the way), it stays
  // whole at the end of the days on screen.
  const admin = await signInThroughApi(server, ada);
  await signIn(page, ada);
  await inOrder(
    [6, 9, 12, 15, 18, 21, 24].map((day) => async () => {
      await changeIssue(admin, planned.found, {
        start_date: dayFromToday(day),
        due_date: dayFromToday(day),
      });
      await openPlan(page);
      await expect.poll(() => cutByTheEdge(page)).toEqual([]);
    }),
  );
  await admin.dispose();
});

test("plan_03_a_bar_of_no_priority_is_closed_by_its_edge_on_every_side", async ({ page }) => {
  await signIn(page, ada);
  await openPlan(page);
  // The edge of a control on all four sides, so a bar of Not started is seen on the dark days of the
  // dark themes too; only Critical and High draw their wide edge at the start (R5).
  const edges = await barOf(page, planned.waits).evaluate((bar) => {
    const look = getComputedStyle(bar);
    return {
      start: [look.borderInlineStartColor, look.borderInlineStartWidth],
      end: [look.borderInlineEndColor, look.borderInlineEndWidth],
    };
  });
  expect(edges.start).toEqual(edges.end);
});
