import { timed } from "../../app/e2e/support/perf.ts";
import { type Context, type Scenario } from "./scenarios.ts";
import { timelineScenarios } from "./timeline-scenarios.ts";

// The scenarios of the totals of a search on the large dataset (QRY-08, PERF-01): the count and the
// lengths of the biggest project by priority, and its total of *Time spent*, which reads through the
// statement of M6-F186. The list of `scenarios.ts` spreads them with those of the timeline, the other
// view of a search measured on the biggest project (`searchViewScenarios`).

const RUNS = 200;

/** The totals of a search, as a box *Numbers and chart* or a grouped list asks for them. */
async function searchTotals(
  context: Context,
  body: { readonly q: string; readonly by?: string; readonly sums: readonly string[] },
): Promise<number> {
  return (
    await timed(() =>
      context.session.client.POST("/api/v1/search/totals", {
        body: { q: body.q, by: body.by ?? null, sums: [...body.sums] },
      }),
    )
  ).ms;
}

const totalsScenarios: readonly Scenario[] = [
  {
    name: "totals-by-priority-biggest-project",
    budget: "search",
    runs: RUNS,
    once: (context) =>
      searchTotals(context, {
        q: `project within ${context.big}`,
        by: "priority",
        sums: ["count", "estimated", "time_left"],
      }),
  },
  {
    name: "totals-time-spent-biggest-project",
    budget: "search",
    runs: RUNS,
    once: (context) =>
      searchTotals(context, { q: `project within ${context.big}`, sums: ["time_spent"] }),
  },
];

/** The scenarios of the views of a search: the timeline and the totals. */
export const searchViewScenarios: readonly Scenario[] = [...timelineScenarios, ...totalsScenarios];
