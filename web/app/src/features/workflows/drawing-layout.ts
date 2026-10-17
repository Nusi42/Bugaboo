/** A box of the drawing, measured from the top left corner of the drawing. */
export interface Box {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

/** A point of the drawing. */
export interface Point {
  readonly x: number;
  readonly y: number;
}

/** An arrow: the path to draw and the middle of it, where its label sits. */
export interface Arrow {
  readonly path: string;
  readonly middle: Point;
}

/** The head of an arrow: a small triangle that the line ends in (the words of the SVG standard). */
export const arrowHead = {
  viewBox: "0 0 10 10",
  refX: 8,
  refY: 5,
  markerWidth: 8,
  markerHeight: 8,
  orient: "auto",
  path: "M 0 0 L 10 5 L 0 10 z",
} as const;

/** The room a label takes on an arrow: the button with the words of the status change. */
export const labelBox = { width: 160, height: 40 } as const;

/** How far an arrow between two boxes of one column bulges out to the side, in the units of the drawing. */
export const BULGE = 40;

/** How far apart two arrows run when a status change and its way back join the same two boxes. */
export const LANE_GAP = 14;

/** How strongly an arrow between two columns bends out of its box before it turns to the other. */
const BEND_SHARE = 0.4;

/** The least distance between the left sides of two boxes for them to count as different columns. */
const COLUMN_GAP = 8;

/** The middle of a side of a box, moved down by `lane` steps so that two arrows do not lie on each other. */
function side(box: Box, which: "left" | "right", lane: number): Point {
  return {
    x: which === "left" ? box.left : box.left + box.width,
    y: box.top + box.height / 2 + lane * LANE_GAP,
  };
}

/** The arrow between two boxes in different columns: from the side of the first that faces the second. */
function acrossColumns(from: Box, to: Box, lane: number): Arrow {
  const toTheRight = to.left > from.left;
  const start = side(from, toTheRight ? "right" : "left", lane);
  const end = side(to, toTheRight ? "left" : "right", lane);
  const bend = Math.abs(end.x - start.x) * BEND_SHARE * (toTheRight ? 1 : -1);
  return {
    path: `M ${start.x} ${start.y} C ${start.x + bend} ${start.y} ${end.x - bend} ${end.y} ${end.x} ${end.y}`,
    middle: { x: (start.x + end.x) / 2, y: (start.y + end.y) / 2 },
  };
}

/** The arrow between two boxes of one column: out of the right sides, round the bulge, and back in. */
function withinColumn(from: Box, to: Box, lane: number): Arrow {
  const start = side(from, "right", lane);
  const end = side(to, "right", lane);
  const out = Math.max(start.x, end.x) + BULGE;
  return {
    path: `M ${start.x} ${start.y} C ${out} ${start.y} ${out} ${end.y} ${end.x} ${end.y}`,
    middle: { x: out - BULGE / 4, y: (start.y + end.y) / 2 },
  };
}

/**
 * The arrow of a status change from one box to another. `lane` moves it aside (a way back between
 * the same two boxes gets the opposite lane), so the arrows can be told apart and followed.
 */
export function arrowOf(from: Box, to: Box, lane = 0): Arrow {
  return Math.abs(to.left - from.left) < COLUMN_GAP
    ? withinColumn(from, to, lane)
    : acrossColumns(from, to, lane);
}
