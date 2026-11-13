import { pixelsOf, usePlace } from "@bugaboo/ui";
import { useRef } from "react";

import { useDropOf } from "../../shared/drop-of.ts";
import { addDays } from "../../shared/format.ts";
import { dragKinds } from "./drag-kinds.ts";
import { useTimeline } from "./timeline-context.ts";

/**
 * The box of the chart: as wide as its days and as high as its rows, at the width of a day of its
 * zoom. An issue of *Not scheduled yet* dropped on it starts on the day it was let go.
 */
export function useCanvas(rows: number) {
  const timeline = useTimeline();
  const canvas = useRef<HTMLDivElement>(null);
  usePlace(canvas, {
    "timeline-day": timeline.dayPixels,
    "timeline-days": timeline.span.days,
    "timeline-rows": rows,
  });
  const drop = useDropOf(dragKinds.unscheduled.kind, canvas, true, (issue, at) => {
    const day = Math.floor((at.x - pixelsOf("size-timeline-names")) / timeline.dayPixels);
    if (day >= 0 && day < timeline.span.days) {
      timeline.onPlace(issue, addDays(timeline.span.first, day));
    }
  });
  return {
    canvas,
    drop,
    rowPixels: timeline.rowPixels,
    dayPixels: timeline.dayPixels,
    first: timeline.span.first,
  };
}
