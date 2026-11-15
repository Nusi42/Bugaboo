import { describe, expect, it } from "vitest";

import { judged } from "./movement-frames.ts";
import type { Box } from "./movement-record.ts";

/** A box at `x`, `y` of `width` by `height`. */
function box(x: number, y: number, width: number, height: number): Box {
  return { x, y, width, height };
}

describe("ux_01 the measure tells a row that left the view from one drawn afresh", () => {
  const shown = box(0, 0, 300, 44);
  /** Three frames of one landmark: there, then gone, with or without the same thing drawn afresh. */
  const frames = (redrawn: boolean) => ({
    boxes: [[shown], [null], [null]],
    redrawn: [[false], [redrawn], [false]],
    scrolls: ["page:0", "page:0", "page:0"],
    grey: [false, false, false],
    view: 800,
  });

  it("does not count a row of a list gone from the view as gone", () => {
    expect(judged([["row", true]], frames(false)).vanished).toEqual([]);
  });

  it("counts a row drawn afresh as gone", () => {
    expect(judged([["row", true]], frames(true)).vanished).toEqual(["row"]);
  });

  it("counts any other part that is no longer there as gone", () => {
    expect(judged([["head", false]], frames(false)).vanished).toEqual(["head"]);
  });
});

describe("ux_01 a landmark held in place by the browser shows no scroll was kept", () => {
  const head = box(0, 0, 300, 44);
  /** Two frames of one landmark that stood still, pinned in place (sticky or fixed) or not. */
  const frames = (pinned: boolean) => ({
    boxes: [[head], [head]],
    redrawn: [[false], [false]],
    scrolls: ["page:0", "page:0"],
    grey: [false, false],
    view: 800,
    pinned: [pinned],
  });

  it("counts a landmark that stood still where it flows", () => {
    expect(judged([["head", false]], frames(false)).still).toEqual([head]);
  });

  it("does not count a sticky or fixed landmark as one that stood still", () => {
    expect(judged([["head", false]], frames(true)).still).toEqual([]);
  });
});
