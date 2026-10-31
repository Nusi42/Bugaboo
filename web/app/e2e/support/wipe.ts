import { type Frame } from "./gif.ts";
import { HERO } from "./readme-pace.ts";

// The front picture of README.md shows one screen in the four themes, one after the other: the next
// theme wipes in over the last along a slanted edge. Each step of a wipe changes only a band of
// pixels, so the GIF, which writes only what changed, stays small.

/** How far right the edge leans for every pixel it goes down. */
const SLANT = 0.5;

const CHANNELS = 4;

/**
 * `from` with `to` shown up to `progress` (0 is only `from`, 1 is only `to`), along an edge that
 * leans from the top left to the bottom right; the step shows for `delayMs`.
 */
export function diagonalWipe(from: Frame, to: Frame, progress: number, delayMs: number): Frame {
  if (from.width !== to.width || from.height !== to.height) {
    throw new RangeError("A wipe needs two pictures of the same size.");
  }
  const { width, height } = from;
  const edge = progress * (width + height * SLANT);
  const rgba = Buffer.from(from.rgba);
  for (let y = 0; y < height; y += 1) {
    // The pixels of this row left of the edge come from `to`.
    const reach = Math.min(width, Math.max(0, Math.ceil(edge - y * SLANT)));
    const start = y * width * CHANNELS;
    to.rgba.copy(rgba, start, start, start + reach * CHANNELS);
  }
  return { width, height, rgba, delayMs };
}

/** The moments of a wipe of `steps` steps, between its start (0) and its end (1). */
export function wipeSteps(steps: number): readonly number[] {
  return Array.from({ length: steps }, (_, step) => (step + 1) / (steps + 1));
}

/**
 * The frames of a cycle through some pictures: each one shows for `holdMs`, then the next one
 * wipes in over it in `steps` steps of `stepMs`; after the last, the first wipes in again, so the
 * GIF can play again and again without a jump. The times are those of the front picture (`HERO`)
 * unless they are given.
 */
export function wipeCycle(
  pictures: readonly Frame[],
  holdMs: number = HERO.holdMs,
  steps: number = HERO.steps,
  stepMs: number = HERO.stepMs,
): readonly Frame[] {
  return pictures.flatMap((picture, number) => {
    const next = pictures[(number + 1) % pictures.length] ?? picture;
    return [
      { ...picture, delayMs: holdMs },
      ...wipeSteps(steps).map((progress) => diagonalWipe(picture, next, progress, stepMs)),
    ];
  });
}
