import { part, partOf, pixelsOf } from "@bugaboo/ui";

import type { Arrow } from "./lines.ts";
import { useTimeline } from "./timeline-context.ts";

/** The size of the head of an arrow, how far before its end an arrow leaves a bar, and the room above and below a bar. */
function arrowSizes(): { readonly head: number; readonly step: number; readonly inset: number } {
  return { head: pixelsOf("space-1"), step: pixelsOf("space-2"), inset: pixelsOf("space-1") };
}

/**
 * The path of one arrow in pixels of the chart: out of the bottom (or the top) of the bar that must
 * finish first, a little before its end, so it never crosses the Issue ID after the bar; along the
 * line between the rows, never across a bar; down or up beside the start of the bar that waits; and
 * into its left edge, where its head ends.
 */
function pathOf(
  arrow: Arrow,
  day: number,
  row: number,
): { readonly line: string; readonly head: string } {
  const { head, step, inset } = arrowSizes();
  const down = arrow.to.line >= arrow.from.line;
  const x1 = Math.max(arrow.from.day * day - step, (arrow.from.day - 1) * day);
  const y1 = down ? (arrow.from.line + 1) * row - inset : arrow.from.line * row + inset;
  const between = down ? arrow.to.line * row : (arrow.to.line + 1) * row;
  const [x2, y2] = [arrow.to.day * day, (arrow.to.line + 0.5) * row];
  const elbow = x2 - step;
  return {
    line: `M ${x1} ${y1} V ${between} H ${elbow} V ${y2} H ${x2 - head * 2}`,
    head: `M ${x2} ${y2} l ${-head * 2} ${-head} v ${head * 2} z`,
  };
}

/**
 * The dependencies of the rows on screen as arrows (PLAN-02, UX §4.7), from the issue that must
 * finish first to the one that waits. They are relations of the server; this draws them only.
 */
export function Arrows({ arrows }: { readonly arrows: readonly Arrow[] }) {
  const { dayPixels, rowPixels } = useTimeline();
  return (
    <svg {...partOf(part.timelineArrows)} aria-hidden>
      {arrows.map((arrow) => {
        const path = pathOf(arrow, dayPixels, rowPixels);
        return (
          <g key={arrow.key} data-arrow={arrow.key}>
            <path {...partOf(part.timelineArrowsLine)} d={path.line} />
            <path {...partOf(part.timelineArrowsHead)} d={path.head} />
          </g>
        );
      })}
    </svg>
  );
}
