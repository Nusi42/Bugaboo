import { PACE } from "./readme-pace.ts";

// The way the drawn pointer of a recording of the front page takes from one place to the next.

/** A place on the page, in pixels of the page. */
export interface Point {
  readonly x: number;
  readonly y: number;
}

/** Slow at the start and at the end, as a hand moves: 0 to 1. */
export function eased(share: number): number {
  return share < 0.5 ? 4 * share ** 3 : 1 - (-2 * share + 2) ** 3 / 2;
}

/** The places the pointer passes on its way from `from` to `to`, without `from`, ending at `to`. */
export function glide(from: Point, to: Point, steps: number = PACE.moveSteps): readonly Point[] {
  return Array.from({ length: steps }, (_, step) => {
    const share = eased((step + 1) / steps);
    return { x: from.x + (to.x - from.x) * share, y: from.y + (to.y - from.y) * share };
  });
}
