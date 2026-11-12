import { part, partOf, pixelsOf } from "@bugaboo/ui";

import type { Arrow } from "./lines.ts";
import { useTimeline } from "./timeline-context.ts";

/** The size of the head of an arrow and of the step out of a bar, from the theme file. */
function arrowSizes(): { readonly head: number; readonly step: number } {
  return { head: pixelsOf("space-1"), step: pixelsOf("space-2") };
}

/**
 * The path of one arrow in pixels of the chart: out of the end of the bar that must finish first,
 * down or up to the row of the one that waits, and into its start; and the head at its end.
 */
function pathOf(
  arrow: Arrow,
  day: number,
  row: number,
): { readonly line: string; readonly head: string } {
  const { head, step } = arrowSizes();
  const [x1, y1] = [arrow.from.day * day, (arrow.from.line + 0.5) * row];
  const [x2, y2] = [arrow.to.day * day, (arrow.to.line + 0.5) * row];
  const bend = x1 + step;
  return {
    line: `M ${x1} ${y1} H ${bend} V ${y2} H ${x2}`,
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
