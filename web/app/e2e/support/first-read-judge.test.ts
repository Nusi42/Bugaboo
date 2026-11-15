import { describe, expect, it } from "vitest";

import { movedOnFirstRead, type Track } from "./first-read-judge.ts";
import type { Box } from "./movement-record.ts";

/** A box of the given place and size. */
function box(x: number, y: number, width: number, height: number): Box {
  return { x, y, width, height };
}

/** A part drawn in these boxes, one after the other. */
function track(name: string, ...boxes: readonly (Box | null)[]): Track {
  return { name, boxes: boxes.map((each, frame) => ({ frame, box: each })) };
}

const VIEW = { width: 1280, height: 800 };

describe("ux_01 once drawn on its first read, nothing moves", () => {
  it("passes a part drawn once and left where it is", () => {
    expect(movedOnFirstRead([track("row", null, box(0, 100, 300, 40))], VIEW)).toEqual([]);
  });

  it("fails a part pushed down after it was drawn, by a line that came in above it", () => {
    const row = track("row", box(0, 100, 300, 40), box(0, 132, 300, 40));
    expect(movedOnFirstRead([row], VIEW).map((each) => each.name)).toEqual(["row"]);
  });

  it("passes a part that grows at its end, its start where it was", () => {
    const list = track("list", box(0, 100, 300, 40), box(0, 100, 300, 400), box(0, 100, 320, 400));
    expect(movedOnFirstRead([list], VIEW)).toEqual([]);
  });

  it("fails a part that gets smaller, or moves sideways", () => {
    const shrinks = track("text", box(0, 100, 300, 40), box(0, 100, 300, 20));
    const sideways = track("count", box(10, 100, 30, 20), box(40, 100, 30, 20));
    expect(movedOnFirstRead([shrinks, sideways], VIEW).map((each) => each.name)).toEqual([
      "text",
      "count",
    ]);
  });

  it("passes grey shapes that go, and the content drawn where they stood", () => {
    const grey = track("skeleton", box(0, 100, 300, 40), null);
    const content = track("row", null, box(0, 100, 300, 40));
    expect(movedOnFirstRead([grey, content], VIEW)).toEqual([]);
  });

  it("does not judge a part that was never in view", () => {
    const below = track("row", box(0, 900, 300, 40), box(0, 940, 300, 40));
    expect(movedOnFirstRead([below], VIEW)).toEqual([]);
  });

  it("says where a part was first drawn, where it went and in which frame", () => {
    const row = track("row", box(0, 100, 300, 40), box(0, 100, 300, 40), box(0, 132, 300, 40));
    expect(movedOnFirstRead([row], VIEW)).toEqual([
      { name: "row", from: box(0, 100, 300, 40), to: box(0, 132, 300, 40), frame: 2 },
    ]);
  });
});
