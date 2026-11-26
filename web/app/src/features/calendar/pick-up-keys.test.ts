import { describe, expect, it } from "vitest";

import { stepOfKey } from "./pick-up-keys.ts";

describe("plan_09_the_keys_that_carry_an_issue", () => {
  it("moves a day with the left and right arrows, a week with the up and down arrows", () => {
    expect(stepOfKey("ArrowLeft", false)).toBe(-1);
    expect(stepOfKey("ArrowRight", false)).toBe(1);
    expect(stepOfKey("ArrowUp", false)).toBe(-7);
    expect(stepOfKey("ArrowDown", false)).toBe(7);
  });

  it("moves a week with Shift and the left and right arrows", () => {
    expect(stepOfKey("ArrowLeft", true)).toBe(-7);
    expect(stepOfKey("ArrowRight", true)).toBe(7);
  });

  it("moves nothing for any other key", () => {
    expect(stepOfKey("Enter", false)).toBeUndefined();
    expect(stepOfKey("a", true)).toBeUndefined();
  });
});
