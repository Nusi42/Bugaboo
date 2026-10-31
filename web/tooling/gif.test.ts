import { describe, expect, it } from "vitest";

import {
  codeFrame,
  drawPointer,
  encodeGif,
  type Frame,
  withoutRepeats,
} from "../app/e2e/support/gif.ts";

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

/** A picture whose rows are 64 close shades of one tint, like the soft edges of letters on a light row. */
function shades(): Frame {
  const rgba = Buffer.alloc(SIZE * SIZE * 4, 255);
  for (let row = 0; row < SIZE; row += 1) {
    for (let column = 0; column < SIZE; column += 1) {
      rgba.set(
        [200 + (row >> 1), 220 + (row >> 2), 250 - (row >> 1), 255],
        (row * SIZE + column) * 4,
      );
    }
  }
  return { width: SIZE, height: SIZE, rgba, delayMs: 100 };
}

/** The color a coded pixel is shown in. */
function shownColor(coded: ReturnType<typeof codeFrame>, pixel: number): readonly number[] {
  return coded.palette[coded.indices[pixel] ?? 0] ?? [];
}

/** How far the shown colors of a coded frame are from the colors of the frame, at most, in one channel. */
function worstError(frame: Frame, coded: ReturnType<typeof codeFrame>): number {
  let worst = 0;
  for (let pixel = 0; pixel < SIZE * SIZE; pixel += 1) {
    const shown = shownColor(coded, pixel);
    for (let channel = 0; channel < 3; channel += 1) {
      const wanted = frame.rgba[pixel * 4 + channel] ?? 0;
      worst = Math.max(worst, Math.abs(wanted - (shown[channel] ?? 0)));
    }
  }
  return worst;
}

/** The red, green and blue of a pixel of a frame. */
function colorAt(frame: Frame, x: number, y: number): readonly number[] {
  const at = (y * frame.width + x) * 4;
  return [...frame.rgba.subarray(at, at + 3)];
}

describe("withoutRepeats", () => {
  it("joins pictures that are the same into one that shows longer", () => {
    const frames = withoutRepeats([picture(5), picture(5), picture(6), picture(6), picture(6)]);
    expect(frames.map((frame) => frame.delayMs)).toEqual([200, 300]);
  });
});

describe("codeFrame", () => {
  it("keeps close shades apart, so a light row does not band", () => {
    const frame = shades();
    // Four bits a channel are off by 14 here.
    expect(worstError(frame, codeFrame(frame))).toBeLessThanOrEqual(7);
  });

  it("shows the most common color of a frame exactly, so a white page stays white", () => {
    // A white page, and as many pixels of a few shades so close to white that the table puts
    // them together with it.
    const rgba = Buffer.alloc(SIZE * SIZE * 4, 255);
    for (let pixel = 0; pixel < (SIZE * SIZE) / 3; pixel += 1) {
      rgba.set([248 + (pixel % 4), 249 + (pixel % 3), 248 + (pixel % 5)], pixel * 4);
    }
    const coded = codeFrame({ width: SIZE, height: SIZE, rgba, delayMs: 100 });
    expect(shownColor(coded, SIZE * SIZE - 1)).toEqual([255, 255, 255]);
  });

  it("chooses the colors of a frame from what changed, and marks the rest unchanged", () => {
    const first = picture(5);
    const second = picture(5);
    second.rgba.set([250, 120, 10, 255], (40 * SIZE + 40) * 4);
    const coded = codeFrame(second, first);
    expect(shownColor(coded, 40 * SIZE + 40)).toEqual([250, 120, 10]);
    expect(coded.indices[0]).toBe(coded.unchanged);
    expect(coded.indices.filter((index) => index !== coded.unchanged)).toHaveLength(1);
  });

  it("gives a frame a table as small as its colors allow, with the last place for unchanged", () => {
    const pointerOnly = picture(5);
    pointerOnly.rgba.set([20, 24, 32, 255], 0);
    const small = codeFrame(pointerOnly, picture(5));
    expect(small.palette).toHaveLength(2);
    expect(small.unchanged).toBe(1);
    const many = codeFrame(shades());
    expect(many.palette.length).toBeGreaterThan(8);
    expect(Math.log2(many.palette.length) % 1).toBe(0);
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

  it("refuses an empty recording and frames of different sizes", () => {
    expect(() => encodeGif([])).toThrow(RangeError);
    const wide = { ...picture(5), width: SIZE * 2, rgba: Buffer.alloc(SIZE * SIZE * 8, 0) };
    expect(() => encodeGif([picture(5), wide])).toThrow(RangeError);
  });
});

describe("drawPointer", () => {
  it("draws the tip of the arrow at the place and stays inside the picture", () => {
    const frame = picture(5);
    drawPointer(frame, 10, 10);
    expect(colorAt(frame, 10, 10)).toEqual([20, 24, 32]);
    expect(() => {
      drawPointer(frame, SIZE - 2, SIZE - 2);
      drawPointer(frame, -5, -5);
    }).not.toThrow();
  });

  it("draws the arrow larger on a sharp screen, at the same place of the page", () => {
    const frame = picture(40);
    drawPointer(frame, 10, 10, 2);
    expect(colorAt(frame, 20, 20)).toEqual([20, 24, 32]);
    expect(colorAt(frame, 21, 21)).toEqual([20, 24, 32]);
    // The white edge right of the tip is two pixels wide too.
    expect(colorAt(frame, 22, 20)).toEqual([255, 255, 255]);
    expect(colorAt(frame, 19, 19)).toEqual([240, 240, 240]);
  });
});
