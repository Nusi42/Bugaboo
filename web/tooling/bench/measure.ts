import { summary } from "../../app/e2e/support/perf.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";
import { type Context, type Scenario } from "./scenarios.ts";

/** How many runs of a scenario are thrown away first. */
const WARM_UP = 20;

/** The time a search is given (SEC-17): a search over it is answered `search.too-slow`. */
const SEARCH_TIME_MS = 2000;

/** How many searches in a row may be too slow before the rest of the runs count as too slow. */
const TOO_SLOW_IN_A_ROW = 3;

/** Whether the read was refused because it took longer than the time a search is given. */
function tooSlow(error: unknown): boolean {
  return error instanceof Error && error.message.includes("search.too-slow");
}

/**
 * Runs a scenario: the warm-up, then the counted runs; what they took, summed up. A search that
 * takes longer than it is given counts as that time; when the first runs all do, the rest are not
 * run (each would wait the full time) and count the same, so the numbers say it plainly.
 */
export async function measure(
  scenario: Scenario,
  context: Context,
): Promise<Record<string, unknown>> {
  const times: number[] = [];
  let slow = 0;
  let inARow = 0;
  const rounds = (count: number, keep: boolean) =>
    Array.from({ length: count }, (_, round) => async () => {
      if (inARow >= TOO_SLOW_IN_A_ROW) {
        if (keep) {
          times.push(SEARCH_TIME_MS);
          slow += 1;
        }
        return;
      }
      try {
        const ms = await scenario.once(context, round);
        inARow = 0;
        if (keep) {
          times.push(ms);
        }
      } catch (error) {
        if (!tooSlow(error)) {
          throw error;
        }
        inARow += 1;
        if (keep) {
          times.push(SEARCH_TIME_MS);
          slow += 1;
        }
      }
    });
  await inOrder([...rounds(WARM_UP, false), ...rounds(scenario.runs, true)]);
  const { requests, median, p95, p99, slowest } = summary(times);
  return { budget: scenario.budget, requests, p50: median, p95, p99, slowest, tooSlow: slow };
}
