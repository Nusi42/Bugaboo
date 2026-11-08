import { describe, expect, it } from "vitest";

import { centsIn, wholeNumberIn } from "./number.ts";
import { deviceRegion } from "./region.ts";

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

const region = (numberFormat: "point" | "comma" | "space") => ({ ...deviceRegion, numberFormat });

describe("ai_31 an amount is read the way the person writes numbers", () => {
  it("reads euros as cents in each number format", () => {
    expect(centsIn("12.50", region("point"))).toBe(1250);
    expect(centsIn("1,234.5", region("point"))).toBe(123_450);
    expect(centsIn("12,50", region("comma"))).toBe(1250);
    expect(centsIn("1.234,50", region("comma"))).toBe(123_450);
    expect(centsIn("12,5", region("space"))).toBe(1250);
    expect(centsIn("1 234,50", region("space"))).toBe(123_450);
    expect(centsIn(" 3 ", region("point"))).toBe(300);
  });

  it("refuses what is no amount of euros and cents", () => {
    for (const text of [
      "",
      "abc",
      "1.234",
      "-2",
      "1e3",
      "12,50 x",
      "12.5.0",
      "1,2,3",
      "1,23,456",
      "1,2345",
      "12,",
      ",5",
    ]) {
      expect(`${text}: ${String(centsIn(text, region("point")))}`).toBe(`${text}: null`);
    }
  });

  it("takes a group mark only between groups of three digits", () => {
    expect(centsIn("1,234,567", region("point"))).toBe(123_456_700);
    expect(centsIn("1.234.567,5", region("comma"))).toBe(123_456_750);
    expect(centsIn("12,34", region("point"))).toBeNull();
    expect(centsIn("1,2,3", region("point"))).toBeNull();
  });
});
