import { describe, expect, it } from "vitest";

import { arrowOf, BULGE, LANE_GAP, type Box } from "./drawing-layout.ts";

const open: Box = { left: 0, top: 0, width: 100, height: 40 };
const progress: Box = { left: 200, top: 0, width: 100, height: 40 };
const below: Box = { left: 0, top: 100, width: 100, height: 40 };

/** The numbers of a path, in order. */
function numbers(path: string): number[] {
  return path
    .split(" ")
    .map(Number)
    .filter((each) => !Number.isNaN(each));
}

describe("wf_02 the drawing of a workflow joins the boxes with arrows", () => {
  it("goes from the side of a box that faces the other, to the side that faces back", () => {
    const [startX, startY, , , , , endX, endY] = numbers(arrowOf(open, progress).path);
    expect([startX, startY]).toEqual([100, 20]);
    expect([endX, endY]).toEqual([200, 20]);
    const [backX] = numbers(arrowOf(progress, open).path);
    expect(backX).toBe(200);
  });

  it("bulges out to the right between two boxes of one column", () => {
    const widest = Math.max(...numbers(arrowOf(open, below).path));
    expect(widest).toBe(100 + BULGE);
  });

  it("keeps a way back apart from the way there", () => {
    const there = numbers(arrowOf(open, progress, 1).path);
    const back = numbers(arrowOf(progress, open, -1).path);
    expect(there[1]).toBe(20 + LANE_GAP);
    expect(back[1]).toBe(20 - LANE_GAP);
  });
});
