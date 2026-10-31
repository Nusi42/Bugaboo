import { describe, expect, it } from "vitest";

import { type Frame } from "../app/e2e/support/gif.ts";
import { diagonalWipe, wipeCycle, wipeSteps } from "../app/e2e/support/wipe.ts";

const WIDTH = 40;
const HEIGHT = 20;

/** A picture of one color. */
function plain(color: readonly number[]): Frame {
  const rgba = Buffer.alloc(WIDTH * HEIGHT * 4);
  for (let at = 0; at < rgba.length; at += 4) {
    rgba.set([...color, 255], at);
  }
  return { width: WIDTH, height: HEIGHT, rgba, delayMs: 100 };
}

const light = plain([250, 250, 250]);
const dark = plain([20, 20, 30]);

/** The red, green and blue of a pixel. */
function colorAt(frame: Frame, x: number, y: number): readonly number[] {
  const at = (y * frame.width + x) * 4;
  return [...frame.rgba.subarray(at, at + 3)];
}

describe("diagonalWipe", () => {
  it("shows only the first picture at the start and only the second at the end", () => {
    expect(diagonalWipe(light, dark, 0, 50).rgba.equals(light.rgba)).toBe(true);
    expect(diagonalWipe(light, dark, 1, 50).rgba.equals(dark.rgba)).toBe(true);
  });

  it("brings the second picture in from the top left, along a slanted edge", () => {
    const half = diagonalWipe(light, dark, 0.5, 50);
    expect(colorAt(half, 0, 0)).toEqual([20, 20, 30]);
    expect(colorAt(half, WIDTH - 1, HEIGHT - 1)).toEqual([250, 250, 250]);
    // The edge leans: further down, the second picture reaches less far to the right.
    const reach = (y: number): number =>
      Array.from({ length: WIDTH }, (_, x) => colorAt(half, x, y)).filter(
        (color) => color[0] === 20,
      ).length;
    expect(reach(0)).toBeGreaterThan(reach(HEIGHT - 1));
  });

  it("keeps the pictures it is given as they were, and the time it is given", () => {
    const before = Buffer.from(light.rgba);
    const step = diagonalWipe(light, dark, 0.3, 40);
    expect(light.rgba.equals(before)).toBe(true);
    expect(step.delayMs).toBe(40);
  });

  it("refuses two pictures of different sizes", () => {
    const small = {
      ...dark,
      width: WIDTH / 2,
      rgba: dark.rgba.subarray(0, (WIDTH * HEIGHT * 4) / 2),
    };
    expect(() => diagonalWipe(light, small, 0.5, 40)).toThrow(RangeError);
  });
});

describe("wipeSteps", () => {
  it("names the moments between the two pictures, without the start and the end", () => {
    expect(wipeSteps(3)).toEqual([0.25, 0.5, 0.75]);
  });
});

describe("wipeCycle", () => {
  it("holds each picture, then wipes to the next, and from the last back to the first", () => {
    const grey = plain([128, 128, 128]);
    const frames = wipeCycle([light, dark, grey], 2000, 2, 40);
    expect(frames).toHaveLength(9);
    expect(frames.map((frame) => frame.delayMs)).toEqual([
      2000, 40, 40, 2000, 40, 40, 2000, 40, 40,
    ]);
    expect(frames[0]?.rgba.equals(light.rgba)).toBe(true);
    expect(frames[3]?.rgba.equals(dark.rgba)).toBe(true);
    // The last step is on its way from grey back to light.
    const last = frames.at(-1);
    expect(last?.rgba.subarray(0, 3)).toEqual(Buffer.from([250, 250, 250]));
    expect(last?.rgba.subarray(-4, -1)).toEqual(Buffer.from([128, 128, 128]));
  });
});
