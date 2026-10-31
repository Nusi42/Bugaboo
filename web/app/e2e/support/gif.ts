import { createRequire } from "node:module";

import type { Gifenc } from "gifenc";

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

/** A GIF has at most this many colors; the last one stands for "unchanged" (see [`encodeGif`]). */
const COLORS = 256;
const UNCHANGED = COLORS - 1;

/** The most pixels that are looked at to choose the colors, so that a long recording stays quick. */
const PALETTE_SAMPLE_PIXELS = 400_000;

const CHANNELS = 4;

/**
 * The same bytes in a buffer of their own that holds nothing else: `gifenc` reads the whole memory
 * behind the bytes it is given, and a buffer cut out of a larger one would show it the rest.
 */
function alone(bytes: Buffer): Buffer {
  return Buffer.from(Uint8Array.from(bytes).buffer);
}

/** Every few pixels of all frames, as one list, to choose the colors from. */
function sampleOf(frames: readonly Frame[]): Buffer {
  const total = frames.reduce((sum, frame) => sum + frame.rgba.length / CHANNELS, 0);
  const step = Math.max(1, Math.ceil(total / PALETTE_SAMPLE_PIXELS));
  const picked: Buffer[] = [];
  for (const { rgba } of frames) {
    for (let pixel = 0; pixel < rgba.length / CHANNELS; pixel += step) {
      picked.push(rgba.subarray(pixel * CHANNELS, pixel * CHANNELS + CHANNELS));
    }
  }
  return alone(Buffer.concat(picked));
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

/**
 * The indices of the colors of a frame: the palette color nearest to each pixel, and the index of
 * "unchanged" (transparent) for each pixel that is as it was in `before`, which the viewer leaves
 * on the screen (frame diffing).
 */
function indicesOf(
  frame: Frame,
  palette: readonly (readonly number[])[],
  before: Frame | undefined,
): Buffer {
  const indices = Buffer.from(applyPalette(alone(frame.rgba), palette, "rgb444"));
  if (before !== undefined) {
    for (let pixel = 0; pixel < indices.length; pixel += 1) {
      const at = pixel * CHANNELS;
      if (frame.rgba.compare(before.rgba, at, at + CHANNELS, at, at + CHANNELS) === 0) {
        indices[pixel] = UNCHANGED;
      }
    }
  }
  return indices;
}

/**
 * The GIF of a recording. One palette of 255 colors is chosen from all frames. A frame after the
 * first writes only the pixels that changed, and the "unchanged" index for every other pixel
 * (frame diffing): a recording of a screen where little moves is small.
 */
export function encodeGif(recording: readonly Frame[]): Uint8Array {
  const frames = withoutRepeats(recording);
  if (frames.length === 0) {
    throw new RangeError("A recording needs at least one frame.");
  }
  const colors = quantize(sampleOf(frames), UNCHANGED, { format: "rgb444" });
  // The table of colors is complete, so that the last index is free for "unchanged".
  const palette = [...colors, ...Array.from({ length: COLORS - colors.length }, () => [0, 0, 0])];
  const encoder = GIFEncoder();
  frames.forEach((frame, number) => {
    const before = frames[number - 1];
    encoder.writeFrame(
      indicesOf(frame, palette.slice(0, UNCHANGED), before),
      frame.width,
      frame.height,
      {
        ...(before === undefined ? { palette } : {}),
        delay: frame.delayMs,
        transparent: before !== undefined,
        transparentIndex: UNCHANGED,
        dispose: 1,
      },
    );
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

/** Draws the arrow of the pointer into the pixels of a frame, its tip at (`x`, `y`). */
export function drawPointer(frame: Frame, x: number, y: number): void {
  ARROW.forEach((line, row) => {
    Array.from(line).forEach((mark, column) => {
      const [left, top] = [Math.round(x) + column, Math.round(y) + row];
      const color = mark === "#" ? DARK : mark === "." ? LIGHT : undefined;
      if (
        color === undefined ||
        left < 0 ||
        top < 0 ||
        left >= frame.width ||
        top >= frame.height
      ) {
        return;
      }
      const at = (top * frame.width + left) * CHANNELS;
      frame.rgba.set([...color, 255], at);
    });
  });
}
