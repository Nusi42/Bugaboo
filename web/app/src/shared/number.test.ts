import { describe, expect, it } from "vitest";

import { numberIn } from "./number.ts";

describe("cfg_06 a number the person types is checked before it is sent", () => {
  const bounds = { least: 5, most: 60 };

  it("takes a whole number inside the bounds, with spaces around it", () => {
    expect(numberIn("20", bounds)).toBe(20);
    expect(numberIn(" 5 ", bounds)).toBe(5);
    expect(numberIn("60", bounds)).toBe(60);
    expect(numberIn("7")).toBe(7);
  });

  it("refuses what is no number, so no NaN is ever sent", () => {
    for (const text of ["", " ", "abc", "1.5", "1e3", "--1", "0x10", "12 3"]) {
      expect(`${text}: ${String(numberIn(text, bounds))}`).toBe(`${text}: null`);
    }
  });

  it("refuses a number outside the bounds", () => {
    expect(numberIn("4", bounds)).toBeNull();
    expect(numberIn("61", bounds)).toBeNull();
    expect(numberIn("-1", bounds)).toBeNull();
  });
});
