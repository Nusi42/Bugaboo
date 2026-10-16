import { describe, expect, it } from "vitest";

import {
  keyboardStep,
  menuStep,
  parseWindowSizes,
  resizedBy,
  sizeAfterKey,
  sizeAfterStep,
  stepOfArrow,
  windowEdges,
  windowSteps,
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

describe("ux_04 a window is resized without dragging too (WCAG 2.5.7)", () => {
  it("has a step for every way the arrow keys change the size", () => {
    const arrows = ["ArrowRight", "ArrowLeft", "ArrowDown", "ArrowUp"] as const;
    expect(arrows.map((arrow) => stepOfArrow(arrow))).toEqual([
      "wider",
      "narrower",
      "taller",
      "shorter",
    ]);
    expect(windowSteps).toHaveLength(arrows.length);
  });

  it("a step of the size menu is a few keys long, in the same four directions", () => {
    expect(menuStep).toBeGreaterThan(keyboardStep);
    expect(sizeAfterStep(start, "wider", menuStep)).toEqual({ width: 760 + menuStep, height: 500 });
    expect(sizeAfterStep(start, "narrower", menuStep)).toEqual({
      width: 760 - menuStep,
      height: 500,
    });
    expect(sizeAfterStep(start, "taller", menuStep)).toEqual({
      width: 760,
      height: 500 + menuStep,
    });
    expect(sizeAfterStep(start, "shorter", menuStep)).toEqual({
      width: 760,
      height: 500 - menuStep,
    });
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
