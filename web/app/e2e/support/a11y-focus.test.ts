import { describe, expect, it } from "vitest";

import { comesAfter } from "./a11y-focus.ts";

describe("comp_15 the order of the code of two stops of the walk", () => {
  it("puts a later sibling, a later branch and a child after the stop before", () => {
    expect(comesAfter([0, 2], [0, 1])).toBe(true);
    expect(comesAfter([1, 0, 0], [0, 5, 7])).toBe(true);
    expect(comesAfter([0, 1, 3], [0, 1])).toBe(true);
  });

  it("puts an earlier sibling, an earlier branch, the parent and the same stop before it", () => {
    expect(comesAfter([0, 1], [0, 2])).toBe(false);
    expect(comesAfter([0, 5, 7], [1, 0, 0])).toBe(false);
    expect(comesAfter([0, 1], [0, 1, 3])).toBe(false);
    expect(comesAfter([0, 1], [0, 1])).toBe(false);
  });
});
