import { describe, expect, it } from "vitest";

import { sideOfSwipe, SWIPE_DISTANCE_PX, swipeSide, type SwipeMove } from "./swipe.ts";

const both: SwipeMove["sides"] = [swipeSide.start, swipeSide.end];
const far = SWIPE_DISTANCE_PX + 1;

/** A move from the middle of a row, sideways by `across` and down by `down`. */
function move(across: number, down = 0, more: Partial<SwipeMove> = {}): SwipeMove {
  return {
    from: { x: 200, y: 100 },
    to: { x: 200 + across, y: 100 + down },
    rightToLeft: false,
    open: swipeSide.none,
    sides: both,
    ...more,
  };
}

describe("ux_02 a swipe on a row opens the actions of one side", () => {
  it("a finger towards the start pulls in the end's actions, towards the end the start's", () => {
    expect(sideOfSwipe(move(-far))).toBe(swipeSide.end);
    expect(sideOfSwipe(move(far))).toBe(swipeSide.start);
  });

  it("right to left, the sides change places", () => {
    expect(sideOfSwipe(move(-far, 0, { rightToLeft: true }))).toBe(swipeSide.start);
    expect(sideOfSwipe(move(far, 0, { rightToLeft: true }))).toBe(swipeSide.end);
  });

  it("a short move, a scroll and a side without actions decide nothing", () => {
    expect(sideOfSwipe(move(SWIPE_DISTANCE_PX - 1))).toBeUndefined();
    expect(sideOfSwipe(move(far, far + 1))).toBeUndefined();
    expect(sideOfSwipe(move(far, 0, { sides: [swipeSide.end] }))).toBeUndefined();
  });

  it("the other way closes the open side; the same way keeps it", () => {
    expect(sideOfSwipe(move(far, 0, { open: swipeSide.end }))).toBe(swipeSide.none);
    expect(sideOfSwipe(move(-far, 0, { open: swipeSide.end }))).toBeUndefined();
  });
});
