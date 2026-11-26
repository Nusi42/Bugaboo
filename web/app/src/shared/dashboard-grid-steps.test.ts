import { describe, expect, it } from "vitest";

import { rowsCrossed, stepsFrom } from "./dashboard-grid-steps.ts";

/** A grid of rows 72 px high with gaps of 16 px, whose second row grew to 300 px with what its boxes hold. */
const steps = stepsFrom({
  width: 1188,
  columnGap: "12px",
  rowHeight: "minmax(72px, auto)",
  rowGap: "16px",
  rows: "72px 300px 72px 72px",
});

describe("the steps of the grid of a dashboard", () => {
  it("reads the least row out of a row that grows, and the rows as drawn", () => {
    expect(steps.row).toBe(88);
    expect(steps.column).toBe(100);
    expect(steps.rows).toEqual([72, 300, 72, 72]);
  });

  it("counts a row passed by more than half its height as drawn", () => {
    expect(rowsCrossed(steps, 0, 40)).toBe(0);
    expect(rowsCrossed(steps, 0, 50)).toBe(1);
    // Past the first row, the grown row takes 316 px: half of it is not yet a row.
    expect(rowsCrossed(steps, 0, 88 + 150)).toBe(1);
    expect(rowsCrossed(steps, 0, 88 + 160)).toBe(2);
  });

  it("counts upwards from the top edge of a row", () => {
    expect(rowsCrossed(steps, 2, -150)).toBe(0);
    expect(rowsCrossed(steps, 2, -160)).toBe(-1);
    expect(rowsCrossed(steps, 2, -(316 + 50))).toBe(-2);
  });

  it("takes a row past the drawn ones as a row of the least height", () => {
    expect(rowsCrossed(steps, 4, 88 * 2)).toBe(2);
  });
});
