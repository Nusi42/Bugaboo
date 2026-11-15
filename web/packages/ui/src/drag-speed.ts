/** How fast a box scrolls, in pixels a second, while a drag is held deepest in the head over it. */
export const DRAG_SCROLL_MOST = 1200;

/** How fast it scrolls with the pointer just inside the head's lower edge. */
export const DRAG_SCROLL_LEAST = 120;

/** The longest time between frames that counts: after a pause the box takes one such step, not a jump. */
const LONGEST_FRAME_MS = 100;

/**
 * How long a drag counts as held without a word from the browser, which tells of a drag over a part several times a
 * second while it is held there: past this the drag left the window or its source went away.
 */
const HELD_WITHOUT_WORD_MS = 300;

const MS_A_SECOND = 1000;

/**
 * How far the box scrolls in a frame: `depth` is how deep the pointer is in the head (0 at its lower edge, 1 at its
 * top), `elapsed` the time since the last frame. The speed does not depend on how often the browser draws.
 */
export function scrollStepOf(depth: number, elapsed: number): number {
  const share = Math.min(1, Math.max(0, depth));
  const speed = DRAG_SCROLL_LEAST + (DRAG_SCROLL_MOST - DRAG_SCROLL_LEAST) * share;
  return (speed * Math.min(elapsed, LONGEST_FRAME_MS)) / MS_A_SECOND;
}

/** Whether the drag still counts as held, the browser having told of it last at `toldAt` (both in ms). */
export function stillHeld(toldAt: number, now: number): boolean {
  return now - toldAt <= HELD_WITHOUT_WORD_MS;
}
