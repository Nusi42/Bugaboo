import { createRequire } from "node:module";

import type { Gifenc, Palette } from "gifenc";

// The package is written for CommonJS and Node, Vitest and Playwright each import it a little
// differently (what a `default` import is, and whether there are named imports), so it is
// required, which gives the same object everywhere.
const loaded: unknown = createRequire(import.meta.url)("gifenc");

/** Whether what was loaded has the three functions this file uses. */
function isGifenc(value: unknown): value is Gifenc {
  return (
    typeof value === "object" &&
    value !== null &&
    ["GIFEncoder", "quantize", "applyPalette"].every(
      (name) => typeof Reflect.get(value, name) === "function",
    )
  );
}

if (!isGifenc(loaded)) {
  throw new TypeError("The package gifenc does not have the functions that were expected.");
}
const gifenc = loaded;

const { applyPalette, GIFEncoder, quantize } = gifenc;

/** One picture of a recording: its pixels as red, green, blue and alpha bytes, and how long it shows. */
export interface Frame {
  readonly width: number;
  readonly height: number;
  readonly rgba: Buffer;
  readonly delayMs: number;
}

/**
 * A table of colors of a GIF has at most this many colors, and always a power of two; its last one
 * stands for "unchanged" (see [`codeFrame`]).
 */
const COLORS = 256;

/**
 * How finely the colors are told apart: 5 bits of red, 6 of green and 5 of blue. Fewer bits
 * (`rgb444`) band the soft edges of letters and the light tints of rows.
 */
const PRECISION = "rgb565";

const CHANNELS = 4;

/**
 * The same bytes in a buffer of their own that holds nothing else: `gifenc` reads the whole memory
 * behind the bytes it is given, and a buffer cut out of a larger one would show it the rest.
 */
function alone(bytes: Buffer): Buffer {
  return Buffer.from(Uint8Array.from(bytes).buffer);
}

/** The pixels of a frame, one number each (red, green, blue and alpha together). */
function pixelsOf(frame: Frame): Uint32Array {
  return new Uint32Array(alone(frame.rgba).buffer);
}

/**
 * Joins the frames that look like the one before (their time is added to it), because a GIF
 * does not need a picture for a moment in which nothing moves.
 */
export function withoutRepeats(frames: readonly Frame[]): readonly Frame[] {
  const kept: Frame[] = [];
  for (const frame of frames) {
    const last = kept.at(-1);
    if (last?.rgba.equals(frame.rgba) === true) {
      kept[kept.length - 1] = { ...last, delayMs: last.delayMs + frame.delayMs };
    } else {
      kept.push(frame);
    }
  }
  return kept;
}

/** Which pixels of a frame differ from `before`: all of them when there is none. */
export function changedPixels(frame: Frame, before?: Frame): Buffer {
  const now = pixelsOf(frame);
  const changed = Buffer.alloc(now.length, 1);
  if (before !== undefined) {
    const then = pixelsOf(before);
    for (let pixel = 0; pixel < now.length; pixel += 1) {
      changed[pixel] = now[pixel] === then[pixel] ? 0 : 1;
    }
  }
  return changed;
}

/** The colors of the changed pixels only, the ones a frame has to say anything about. */
function changedColors(frame: Frame, changed: Buffer): Buffer {
  const count = changed.reduce((sum, each) => sum + each, 0);
  const colors = Buffer.alloc(Math.max(count, 1) * CHANNELS);
  let next = 0;
  changed.forEach((each, pixel) => {
    if (each === 1) {
      frame.rgba.copy(colors, next * CHANNELS, pixel * CHANNELS, pixel * CHANNELS + CHANNELS);
      next += 1;
    }
  });
  return colors;
}

/** What one frame of a GIF holds: its own table of colors and the index of each pixel in it. */
export interface Coded {
  readonly palette: Palette;
  readonly indices: Buffer;
  /** The index that leaves a pixel as the frame before showed it: the last of the table. */
  readonly unchanged: number;
}

/** The smallest table a GIF can have for these many colors and the "unchanged" mark. */
function tableSize(colors: number): number {
  return 2 ** Math.max(1, Math.ceil(Math.log2(colors + 1)));
}

/**
 * Each color of the table made the color that the most of the pixels it stands for really have.
 * The table puts close colors together and gives them their average: a white page with letters
 * on it would come out a little grey where a frame changed it, and stay white where it did not,
 * so the places where the screen changed before would show as faint patches.
 */
function commonest(frame: Frame, changed: Buffer, indices: Buffer, chosen: Palette): Palette {
  const counts = chosen.map(() => new Map<number, number>());
  const pixels = pixelsOf(frame);
  changed.forEach((each, pixel) => {
    const count = counts[indices[pixel] ?? 0];
    const color = (pixels[pixel] ?? 0) & 0xff_ff_ff;
    if (each === 1 && count !== undefined) {
      count.set(color, (count.get(color) ?? 0) + 1);
    }
  });
  const colors: (readonly number[])[] = [];
  for (const [index, count] of counts.entries()) {
    let [most, best] = [0, -1];
    for (const [color, times] of count) {
      if (times > most) {
        [most, best] = [times, color];
      }
    }
    // The pixels are read as little-endian numbers: red is the lowest byte.
    colors.push(
      best < 0 ? (chosen[index] ?? [0, 0, 0]) : [best & 0xff, (best >> 8) & 0xff, best >> 16],
    );
  }
  return colors;
}

/**
 * The colors of a frame and their indices. Each frame chooses up to 255 colors of its own from the pixels
 * that changed since `before`, so a frame in which a new theme comes in has the colors of that
 * theme, and a frame in which only the pointer moved spends no colors on the rest. The pixels that
 * did not change get the index of "unchanged" (transparent), which leaves the frame before on the
 * screen (frame diffing).
 */
export function codeFrame(frame: Frame, before?: Frame): Coded {
  const changed = changedPixels(frame, before);
  const chosen = quantize(changedColors(frame, changed), COLORS - 1, { format: PRECISION });
  const size = tableSize(chosen.length);
  const unchanged = size - 1;
  const indices = Buffer.from(applyPalette(alone(frame.rgba), chosen, PRECISION));
  const colors = commonest(frame, changed, indices, chosen);
  changed.forEach((each, pixel) => {
    if (each === 0) {
      indices[pixel] = unchanged;
    }
  });
  // The table is filled up to its size, so that its last index is free for "unchanged". A frame in
  // which only the pointer moved needs a table of a few colors, not of 256.
  const palette = [...colors, ...Array.from({ length: size - colors.length }, () => [0, 0, 0])];
  return { palette, indices, unchanged };
}

/**
 * The GIF of a recording, which plays again and again. Every frame has its own table of colors
 * ([`codeFrame`]) and writes only the pixels that changed: a recording of a screen where little
 * moves is small, and its colors stay true.
 */
export function encodeGif(recording: readonly Frame[]): Uint8Array {
  const frames = withoutRepeats(recording);
  if (frames.length === 0) {
    throw new RangeError("A recording needs at least one frame.");
  }
  const encoder = GIFEncoder();
  frames.forEach((frame, number) => {
    const before = frames[number - 1];
    if (before !== undefined && (before.width !== frame.width || before.height !== frame.height)) {
      throw new RangeError("Every frame of a recording has the same size.");
    }
    const { palette, indices, unchanged } = codeFrame(frame, before);
    encoder.writeFrame(indices, frame.width, frame.height, {
      palette,
      delay: frame.delayMs,
      transparent: before !== undefined,
      transparentIndex: unchanged,
      // Keep this frame as the background of the next one.
      dispose: 1,
    });
  });
  encoder.finish();
  return encoder.bytes();
}

/** The arrow of the pointer: `#` is dark, `.` is the white edge, a space is left as it is. */
const ARROW = [
  "#...........",
  "##..........",
  "#.#.........",
  "#..#........",
  "#...#.......",
  "#....#......",
  "#.....#.....",
  "#......#....",
  "#.......#...",
  "#........#..",
  "#.....######",
  "#..#..#.....",
  "#.# #..#....",
  "##  #..#....",
  "#    #..#...",
  "     #..#...",
  "      ##....",
] as const;

const DARK = [20, 24, 32];
const LIGHT = [255, 255, 255];

/**
 * Draws the arrow of the pointer into the pixels of a frame, its tip at (`x`, `y`) of the page.
 * `scale` is how many pixels of the frame one pixel of the page is (2 on a sharp screen); the
 * arrow is drawn that much larger, so it has the same size on the page.
 */
export function drawPointer(frame: Frame, x: number, y: number, scale = 1): void {
  const [tipX, tipY] = [Math.round(x * scale), Math.round(y * scale)];
  ARROW.forEach((line, row) => {
    Array.from(line).forEach((mark, column) => {
      const color = mark === "#" ? DARK : mark === "." ? LIGHT : undefined;
      if (color === undefined) {
        return;
      }
      for (let down = 0; down < scale; down += 1) {
        for (let across = 0; across < scale; across += 1) {
          const [left, top] = [tipX + column * scale + across, tipY + row * scale + down];
          if (left >= 0 && top >= 0 && left < frame.width && top < frame.height) {
            frame.rgba.set([...color, 255], (top * frame.width + left) * CHANNELS);
          }
        }
      }
    });
  });
}
