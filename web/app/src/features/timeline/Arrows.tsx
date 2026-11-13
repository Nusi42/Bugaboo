import { part, partOf, pixelsOf } from "@bugaboo/ui";

import { arrowPath, type ArrowSizes } from "./arrow-path.ts";
import type { Arrow } from "./lines.ts";
import { useTimeline } from "./timeline-context.ts";

/** The size of the head of an arrow, how far before its end an arrow leaves a bar, and the room above and below a bar. */
function arrowSizes(): ArrowSizes {
  return { head: pixelsOf("space-1"), step: pixelsOf("space-2"), inset: pixelsOf("space-1") };
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
        const path = arrowPath(arrow, dayPixels, rowPixels, arrowSizes());
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
