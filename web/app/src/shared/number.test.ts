import { describe, expect, it } from "vitest";

import { wholeNumberIn } from "./number.ts";

describe("cfg_06 a number the person types is checked before it is sent", () => {
  const bounds = { least: 5, most: 60 };

  it("takes a whole number inside the bounds, with spaces around it", () => {
    expect(wholeNumberIn("20", bounds)).toBe(20);
    expect(wholeNumberIn(" 5 ", bounds)).toBe(5);
    expect(wholeNumberIn("60", bounds)).toBe(60);
    expect(wholeNumberIn("7")).toBe(7);
    // Twenty digits do not fit a JavaScript number without losing the last ones: they are refused.
    expect(wholeNumberIn("12345678901234567890")).toBeNull();
    expect(wholeNumberIn("123456789012345")).toBe(123456789012345);
  });

  it("refuses what is no number, so no NaN is ever sent", () => {
    for (const text of ["", " ", "abc", "1.5", "1e3", "--1", "0x10", "12 3"]) {
      expect(`${text}: ${String(wholeNumberIn(text, bounds))}`).toBe(`${text}: null`);
    }
  });

  it("refuses a number outside the bounds", () => {
    expect(wholeNumberIn("4", bounds)).toBeNull();
    expect(wholeNumberIn("61", bounds)).toBeNull();
    expect(wholeNumberIn("-1", bounds)).toBeNull();
  });
});
