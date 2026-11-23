import { describe, expect, it } from "vitest";

import { type Measured, measure } from "./measure.ts";

/** The context a scenario of these tests never reads. */
const context = null;

/** A scenario of `runs` counted runs whose run number `n` (warm-up first) is too slow when `slow(n)`. */
function scenario(runs: number, warmUp: number, slow: (n: number) => boolean): Measured<null> {
  let n = 0;
  return {
    budget: "page",
    runs,
    warmUp,
    once: () => {
      n += 1;
      return slow(n) ? Promise.reject(new Error("search.too-slow")) : Promise.resolve(5);
    },
  };
}

/** A scenario of 10 counted runs after 2 of warm-up whose run number `at` gets no answer; how many runs it started. */
function failingAt(at: number): {
  readonly scenario: Measured<null>;
  readonly started: () => number;
} {
  let n = 0;
  return {
    scenario: {
      budget: "page",
      runs: 10,
      warmUp: 2,
      once: () => {
        n += 1;
        return n < at ? Promise.resolve(5) : Promise.reject(new Error("no answer within 35 s"));
      },
    },
    started: () => n,
  };
}

describe("perf_bench the driver of the benchmarks", () => {
  it("throws away runs of the warm-up that are too slow and still makes every counted run", async () => {
    const result = await measure(
      scenario(10, 5, (n) => n <= 4),
      context,
    );
    expect(result).toMatchObject({ made: 10, skipped: 0, tooSlow: 0 });
  });

  it("writes a scenario whose run fails down as failed instead of ending the driver", async () => {
    const failing = failingAt(5);
    const result = await measure(failing.scenario, context);
    expect(result).toEqual({ budget: "page", made: 2, failed: "no answer within 35 s" });
    expect(failing.started()).toBe(5);
  });

  it("counts the rest of the counted runs as too slow after three of them in a row", async () => {
    const result = await measure(
      scenario(10, 2, (n) => n > 2),
      context,
    );
    expect(result).toMatchObject({ made: 3, skipped: 7, tooSlow: 3 });
  });
});
