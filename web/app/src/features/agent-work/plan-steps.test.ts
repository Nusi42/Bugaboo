import { describe, expect, it } from "vitest";

import { planSteps } from "./plan-steps.ts";

describe("ai_33 the plan of an attempt", () => {
  it("is one step for each line, without its number", () => {
    expect(planSteps("1. Read the check\n2. Cut the lifetime\n3) Add a test")).toEqual([
      "Read the check",
      "Cut the lifetime",
      "Add a test",
    ]);
  });

  it("is split at the numbers when the agent wrote it on one line", () => {
    expect(planSteps("1. Find the check. 2. Cut the lifetime. 3. Add a test.")).toEqual([
      "Find the check.",
      "Cut the lifetime.",
      "Add a test.",
    ]);
  });

  it("is one step when it has no numbers", () => {
    expect(planSteps("Fix it")).toEqual(["Fix it"]);
    expect(planSteps("  ")).toEqual([]);
  });
});
