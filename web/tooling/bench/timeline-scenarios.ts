import { pageSizeDefault } from "@bugaboo/api-client";

import layout from "../../../tools/dev/sample.json" with { type: "json" };
import { timed } from "../../app/e2e/support/perf.ts";
import { dayFromToday } from "../../app/e2e/support/timeline.ts";
import { type Context, type Scenario } from "./scenarios.ts";

// The scenarios of the timeline on the large dataset (M8 WP-12, PLAN-01, PLAN-12, PERF-01),
// registered in the list of `scenarios.ts`, which spreads this one. Their data is made by the large
// dataset itself: a share of the issues of the biggest project has a start date, a due date and an
// estimate (`large.datedPercent` of `tools/dev/sample.json`).

const RUNS = 200;

/** How many days before and after the dated issues of the dataset the timeline looks. */
const ROOM_DAYS = 60;

/**
 * A page of the timeline of the biggest project: the rows with their bars, the dependencies and the
 * warnings, worked out once on the server, with or without *Plan by effort*. The span lies around the
 * days the dataset dates a share of the issues of that project to.
 */
async function timelinePage(context: Context, byEffort: boolean): Promise<number> {
  return (
    await timed(() =>
      context.session.client.POST("/api/v1/timeline", {
        body: {
          search_text: "",
          project: context.big,
          timeline: { start_field: null, end_field: null, plan_by_effort: byEffort, zoom: "week" },
          first: dayFromToday(-ROOM_DAYS),
          last: dayFromToday(layout.large.datedDays + ROOM_DAYS),
          limit: pageSizeDefault,
        },
      }),
    )
  ).ms;
}

export const timelineScenarios: readonly Scenario[] = [
  {
    // The timeline route for the biggest project, a page of rows (PLAN-01).
    name: "timeline-biggest-project",
    budget: "timeline",
    runs: RUNS,
    once: (context) => timelinePage(context, false),
  },
  {
    // The same with *Plan by effort*: the bars are worked out from work times, shares and holidays.
    name: "timeline-biggest-project-by-effort",
    budget: "timeline",
    runs: RUNS,
    once: (context) => timelinePage(context, true),
  },
];
