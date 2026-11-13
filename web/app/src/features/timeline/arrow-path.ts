import type { Arrow } from "./lines.ts";

/** The size of the head of an arrow, how far before its end an arrow leaves a bar, and the room above and below a bar. */
export interface ArrowSizes {
  readonly head: number;
  readonly step: number;
  readonly inset: number;
}

/**
 * The path of one arrow in pixels of the chart: out of the bottom (or the top) of the bar that must
 * finish first, a little before its end, so it never crosses the Issue ID after the bar; along the
 * line between the rows, never across a bar; down or up beside the start of the bar that waits; and
 * into its left edge, where its head ends. An arrow whose way along the rows is shorter than its
 * turn and its head goes straight down or up to its head, as a turn would draw a small box.
 */
export function arrowPath(
  arrow: Arrow,
  day: number,
  row: number,
  sizes: ArrowSizes,
): { readonly line: string; readonly head: string } {
  const { head, step, inset } = sizes;
  const down = arrow.to.line >= arrow.from.line;
  const x1 = Math.max(arrow.from.day * day - step, (arrow.from.day - 1) * day);
  const y1 = down ? (arrow.from.line + 1) * row - inset : arrow.from.line * row + inset;
  const between = down ? arrow.to.line * row : (arrow.to.line + 1) * row;
  const [x2, y2] = [arrow.to.day * day, (arrow.to.line + 0.5) * row];
  const tail = x2 - head * 2;
  const headPath = `M ${x2} ${y2} l ${-head * 2} ${-head} v ${head * 2} z`;
  if (Math.abs(tail - x1) < step + head * 2) {
    return { line: `M ${tail} ${y1} V ${y2}`, head: headPath };
  }
  const elbow = x2 - step;
  return { line: `M ${x1} ${y1} V ${between} H ${elbow} V ${y2} H ${tail}`, head: headPath };
}
