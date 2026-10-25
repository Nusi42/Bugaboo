import { summary } from "../../app/e2e/support/perf.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";
import { type Context, type Scenario } from "./scenarios.ts";

/** How many runs of a scenario are thrown away first. */
const WARM_UP = 20;

/** Runs a scenario: the warm-up, then the counted runs; what they took, summed up. */
export async function measure(
  scenario: Scenario,
  context: Context,
): Promise<Record<string, unknown>> {
  const times: number[] = [];
  const rounds = (count: number, keep: boolean) =>
    Array.from({ length: count }, (_, round) => async () => {
      const ms = await scenario.once(context, round);
      if (keep) {
        times.push(ms);
      }
    });
  await inOrder([...rounds(WARM_UP, false), ...rounds(scenario.runs, true)]);
  const { requests, median, p95, p99, slowest } = summary(times);
  return { budget: scenario.budget, requests, p50: median, p95, p99, slowest };
}
