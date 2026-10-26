import { summary } from "../../app/e2e/support/perf.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";
import { type Context, type Scenario } from "./scenarios.ts";

/** How many runs of a scenario are thrown away first. */
const WARM_UP = 20;

/** How many searches in a row may be too slow before the rest of the runs count as too slow. */
const TOO_SLOW_IN_A_ROW = 3;

/** Whether the read was refused because it took longer than the time a search is given. */
function tooSlow(error: unknown): boolean {
  return error instanceof Error && error.message.includes("search.too-slow");
}

/** What the counted runs of one scenario took, and how many were too slow or not made. */
class Tally {
  readonly times: number[] = [];
  slow = 0;
  skipped = 0;
  inARow = 0;
  slowest = 0;

  /** Whether the runs that are left are not made, because the last ones were all too slow. */
  get giveUp(): boolean {
    return this.inARow >= TOO_SLOW_IN_A_ROW;
  }

  /** A run that is not made counts as the slowest time seen. */
  skip(): void {
    this.times.push(this.slowest);
    this.slow += 1;
    this.skipped += 1;
  }

  /** A run that took `ms`; `stopped` when the server stopped it for taking too long. */
  record(ms: number, stopped: boolean, counted: boolean): void {
    this.inARow = stopped ? this.inARow + 1 : 0;
    this.slowest = Math.max(this.slowest, stopped ? ms : 0);
    if (counted) {
      this.times.push(ms);
      this.slow += stopped ? 1 : 0;
    }
  }

  /** One run of a scenario into the tally. */
  async run(scenario: Scenario, context: Context, round: number, counted: boolean): Promise<void> {
    if (this.giveUp) {
      if (counted) {
        this.skip();
      }
      return;
    }
    const started = performance.now();
    try {
      this.record(await scenario.once(context, round), false, counted);
    } catch (error) {
      if (!tooSlow(error)) {
        throw error;
      }
      this.record(performance.now() - started, true, counted);
    }
  }
}

/**
 * Runs a scenario: the warm-up, then the counted runs; what they took, summed up.
 *
 * A search that the server stops for taking too long counts as the time it really took (at least
 * the limit, which the server owns and this file does not name). Once three runs in a row are too
 * slow, wherever in the run, the rest are not made (each would wait the full time) and count as the
 * slowest time seen, so the numbers say plainly that the budget is far over. `made` and `skipped`
 * tell how many counted runs were made and how many were counted without one.
 */
export async function measure(
  scenario: Scenario,
  context: Context,
): Promise<Record<string, unknown>> {
  const tally = new Tally();
  const rounds = (count: number, counted: boolean) =>
    Array.from({ length: count }, (_, round) => () => tally.run(scenario, context, round, counted));
  await inOrder([...rounds(WARM_UP, false), ...rounds(scenario.runs, true)]);
  const { median, p95, p99, slowest } = summary(tally.times);
  return {
    budget: scenario.budget,
    made: tally.times.length - tally.skipped,
    skipped: tally.skipped,
    p50: median,
    p95,
    p99,
    slowest,
    tooSlow: tally.slow,
  };
}
