import { describe, expect, it } from "vitest";

import { limitProblem, limitText, limitsOf } from "./card-limits.ts";

describe("the card limits of a board (M10-Q2)", () => {
  it("writes the limit of a column and leaves a column without one empty", () => {
    const limits = [{ column: "In progress", limit: 3 }];
    expect(limitText(limits, "In progress")).toBe("3");
    expect(limitText(limits, "Open")).toBe("");
  });

  it("takes a number from 1 to 999 or nothing", () => {
    expect(limitProblem("")).toBeUndefined();
    expect(limitProblem(" 12 ")).toBeUndefined();
    expect(limitProblem("0")).toBeDefined();
    expect(limitProblem("1000")).toBeDefined();
    expect(limitProblem("three")).toBeDefined();
  });

  it("keeps the limits of the boxes that hold one, for the columns they belong to", () => {
    expect(limitsOf(["Open", "In progress", null], { "In progress": "3", "": " 5 " })).toEqual([
      { column: "In progress", limit: 3 },
      { column: null, limit: 5 },
    ]);
  });
});
