import { dataOf, pageSizeDefault } from "@bugaboo/api-client";

import layout from "../../../tools/dev/sample.json" with { type: "json" };
import { timed } from "../../app/e2e/support/perf.ts";
import { utcDayFromToday } from "./reads.ts";
import { type Context, type Scenario } from "./scenarios.ts";

// The scenarios of the timeline on the large dataset (M8 WP-12, PLAN-01, PLAN-12, PERF-01),
// registered in the list of `scenarios.ts`, which spreads this one. Their data is made by the large
// dataset itself: a share of the issues of the biggest project has a start date, a due date and an
// estimate (`large.datedPercent` of `tools/dev/sample.json`).

const RUNS = 200;

/**
 * The span the timeline looks at, the longest a request may ask for (a year): it reaches back far
 * enough to hold the dated issues of a dataset that is some months old (they start within
 * `large.datedDays` days after the day it was made) and ahead to the end of the year.
 */
const DAYS_BEHIND = 180;
const DAYS_AHEAD = 185;

/** How many bars a page must hold at least: an empty page is never timed. */
const MIN_BARS = 20;

/**
 * A page of the timeline of the biggest project: the rows with their bars, the dependencies and the
 * warnings, worked out once on the server, with or without *Plan by effort*. A page with fewer than
 * `MIN_BARS` bars (a dataset made without dates, or too old) fails the run instead of being timed.
 */
async function timelinePage(context: Context, byEffort: boolean): Promise<number> {
  const asked = await timed(() =>
    context.session.client.POST("/api/v1/timeline", {
      body: {
        search_text: "",
        project: context.big,
        timeline: { start_field: null, end_field: null, plan_by_effort: byEffort, zoom: "week" },
        first: utcDayFromToday(-DAYS_BEHIND),
        last: utcDayFromToday(DAYS_AHEAD),
        limit: pageSizeDefault,
      },
    }),
  );
  const bars = dataOf(asked.value).rows.length;
  if (bars < MIN_BARS) {
    throw new Error(
      `The timeline of ${context.big} has ${String(bars)} bars, fewer than ${String(MIN_BARS)}: make the dataset again (dates start within ${String(layout.large.datedDays)} days of its day).`,
    );
  }
  return asked.ms;
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
