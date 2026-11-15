import { describe, expect, it } from "vitest";

import { DRAG_SCROLL_MOST, DRAG_SCROLL_LEAST, scrollStepOf, stillHeld } from "./drag-speed.ts";

/** The speed of a drag at a depth, in pixels a second, from frames 50 ms apart. */
function perSecond(depth: number): number {
  return scrollStepOf(depth, 50) * 20;
}

describe("ux_06 a drag over the folded head scrolls the box at a speed of its own", () => {
  it("scrolls by the time between frames, the same at 60 and at 120 frames a second", () => {
    const at60 = scrollStepOf(1, 1000 / 60);
    const at120 = scrollStepOf(1, 1000 / 120);
    expect(at60).toBeCloseTo(2 * at120, 6);
    expect(perSecond(1)).toBeCloseTo(DRAG_SCROLL_MOST, 6);
  });

  it("goes faster the deeper the pointer is in the head, never beyond its most or below its least", () => {
    expect(perSecond(0.5)).toBeLessThan(perSecond(1));
    expect(perSecond(3)).toBeCloseTo(DRAG_SCROLL_MOST, 6);
    expect(perSecond(0)).toBeCloseTo(DRAG_SCROLL_LEAST, 6);
  });

  it("takes no step for a frame that came after a long pause, beyond a frame's worth", () => {
    expect(scrollStepOf(1, 5000)).toBeCloseTo(scrollStepOf(1, 100), 6);
  });

  it("counts a drag as held only while the browser keeps telling of it", () => {
    expect(stillHeld(1000, 1100)).toBe(true);
    // The HTML standard has a drag tell of itself every 350 ms, give or take 200 ms.
    expect(stillHeld(1000, 1550)).toBe(true);
    expect(stillHeld(1000, 2000)).toBe(false);
  });
});
