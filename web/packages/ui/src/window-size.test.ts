import { describe, expect, it } from "vitest";

import {
  keyboardStep,
  parseWindowSizes,
  resizedBy,
  sizeAfterKey,
  windowEdges,
} from "./window-size.ts";

const start = { width: 760, height: 500 } as const;
const key = { left: "ArrowLeft", right: "ArrowRight", up: "ArrowUp", down: "ArrowDown" } as const;

describe("ux_6_3 a window is resized at every edge and corner", () => {
  it("has the four edges and the four corners the theme file draws", () => {
    expect([...windowEdges].toSorted((first, second) => first.localeCompare(second))).toEqual([
      "e",
      "n",
      "ne",
      "nw",
      "s",
      "se",
      "sw",
      "w",
    ]);
  });

  it("the edge that is dragged follows the pointer, so the centered window grows on both sides", () => {
    expect(resizedBy(start, "e", { x: 20, y: 7 })).toEqual({ width: 800, height: 500 });
    expect(resizedBy(start, "w", { x: -20, y: 7 })).toEqual({ width: 800, height: 500 });
    expect(resizedBy(start, "s", { x: 5, y: 10 })).toEqual({ width: 760, height: 520 });
    expect(resizedBy(start, "n", { x: 5, y: -10 })).toEqual({ width: 760, height: 520 });
  });

  it("a corner changes both sides, the other way when the pointer goes inward", () => {
    expect(resizedBy(start, "se", { x: 10, y: 10 })).toEqual({ width: 780, height: 520 });
    expect(resizedBy(start, "nw", { x: 10, y: 10 })).toEqual({ width: 740, height: 480 });
    expect(resizedBy(start, "ne", { x: -10, y: 10 })).toEqual({ width: 740, height: 480 });
    expect(resizedBy(start, "sw", { x: -10, y: -10 })).toEqual({ width: 780, height: 480 });
  });
});

describe("ux_04 a window is resized with the keyboard too", () => {
  it("the arrow keys make it wider, narrower, taller and shorter by a step", () => {
    expect(sizeAfterKey(start, key.right)).toEqual({ width: 760 + keyboardStep, height: 500 });
    expect(sizeAfterKey(start, key.left)).toEqual({ width: 760 - keyboardStep, height: 500 });
    expect(sizeAfterKey(start, key.down)).toEqual({ width: 760, height: 500 + keyboardStep });
    expect(sizeAfterKey(start, key.up)).toEqual({ width: 760, height: 500 - keyboardStep });
  });

  it("any other key changes nothing", () => {
    expect(sizeAfterKey(start, "Enter")).toBeUndefined();
  });
});

describe("ux_6_3 the size is remembered per form", () => {
  it("reads the sizes the settings hold and drops what is not a size", () => {
    const text = JSON.stringify({
      "new-issue": { width: 800, height: 600 },
      broken: { width: "wide" },
      negative: { width: -1, height: 400 },
    });
    expect(parseWindowSizes(text)).toEqual({ "new-issue": { width: 800, height: 600 } });
  });

  it("is empty for text that is no object", () => {
    expect(parseWindowSizes("[1, 2]")).toEqual({});
    expect(parseWindowSizes("not json")).toEqual({});
  });
});
