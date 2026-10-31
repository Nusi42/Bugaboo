import { describe, expect, it } from "vitest";

import { drawPointer, encodeGif, type Frame, withoutRepeats } from "../app/e2e/support/gif.ts";

const SIZE = 64;

/** A grey picture of 64 by 64 pixels with a dark square whose left edge is at `square`. */
function picture(square: number, delayMs = 100): Frame {
  const rgba = Buffer.alloc(SIZE * SIZE * 4, 240);
  for (let row = 20; row < 30; row += 1) {
    for (let column = square; column < square + 10; column += 1) {
      rgba.set([30, 60, 200, 255], (row * SIZE + column) * 4);
    }
  }
  return { width: SIZE, height: SIZE, rgba, delayMs };
}

describe("withoutRepeats", () => {
  it("joins pictures that are the same into one that shows longer", () => {
    const frames = withoutRepeats([picture(5), picture(5), picture(6), picture(6), picture(6)]);
    expect(frames.map((frame) => frame.delayMs)).toEqual([200, 300]);
  });
});

describe("encodeGif", () => {
  it("writes a GIF that ends with its trailer", () => {
    const bytes = encodeGif([picture(5), picture(15), picture(25)]);
    expect(Buffer.from(bytes.subarray(0, 6)).toString("latin1")).toBe("GIF89a");
    expect(bytes.at(-1)).toBe(0x3b);
  });

  it("makes a picture that changes in a few places smaller than one that changes everywhere", () => {
    const small = Array.from({ length: 12 }, (_, step) => picture(step));
    const noisy = small.map((frame, step) => {
      const rgba = Buffer.from(frame.rgba);
      for (let at = 0; at < rgba.length; at += 4) {
        rgba.set([(at / 4 + step * 37) % 256, (at / 8 + step * 11) % 256, step * 20, 255], at);
      }
      return { ...frame, rgba };
    });
    expect(encodeGif(small).length * 3).toBeLessThan(encodeGif(noisy).length);
  });

  it("refuses an empty recording", () => {
    expect(() => encodeGif([])).toThrow(RangeError);
  });
});

describe("drawPointer", () => {
  it("draws the tip of the arrow at the place and stays inside the picture", () => {
    const frame = picture(5);
    drawPointer(frame, 10, 10);
    expect([...frame.rgba.subarray((10 * SIZE + 10) * 4, (10 * SIZE + 10) * 4 + 3)]).toEqual([
      20, 24, 32,
    ]);
    expect(() => {
      drawPointer(frame, SIZE - 2, SIZE - 2);
      drawPointer(frame, -5, -5);
    }).not.toThrow();
  });
});
