import { pixelsOf, usePlace } from "@bugaboo/ui";
import { useContext, useState, type RefObject } from "react";

import type { BarDays } from "./lines.ts";
import { placeOf, type Span } from "./scale.ts";
import { ChartViewContext, type TimelineContextValue } from "./timeline-context.ts";
import type { ChartView } from "./use-visible-rows.ts";

/** Puts a bar on its days of the chart, with how much of it is done (the run-time tokens). */
export function useBarPlace(
  ref: Readonly<RefObject<HTMLElement | null>>,
  span: Span,
  shown: BarDays,
  done: number,
): void {
  const start = placeOf(span, shown.first);
  usePlace(ref, {
    "timeline-start": start,
    "timeline-length": placeOf(span, shown.last) - start + 1,
    "timeline-done": done,
  });
}

/**
 * Whether the label of a bar stands before it: there is too little room after the bar for the label
 * as it is drawn (`label`, its measured width), before the end of the chart or of the days on
 * screen, and room for it before the bar, after the start of the days on screen.
 */
function labelGoesBefore(
  timeline: Pick<TimelineContextValue, "span" | "dayPixels">,
  shown: BarDays,
  view: ChartView,
  label: { readonly width: number; readonly selected: boolean },
): boolean {
  // The gap before the label and a margin at the end of what is seen; a selected bar has its dot
  // and its ring between its end and its label.
  const dot = label.selected ? pixelsOf("size-target-least") + pixelsOf("space-1") : 0;
  const room = label.width + pixelsOf("space-2") * 2 + dot;
  const start = placeOf(timeline.span, shown.first) * timeline.dayPixels;
  const end = (placeOf(timeline.span, shown.last) + 1) * timeline.dayPixels;
  const after = Math.min(timeline.span.days * timeline.dayPixels, view.end) - end;
  return after < room && start - Math.max(0, view.start) >= room;
}

/**
 * Where the label of a bar stands (UX §4.7): after its end, or before the bar when it would run past
 * the end of the chart or of the days on screen. The label says how wide it is drawn
 * (`onLabelWidth`), so a short label stays after the bar where a long one would not fit.
 */
export function useLabelPlace(
  timeline: Pick<TimelineContextValue, "span" | "dayPixels">,
  shown: BarDays,
  selected: boolean,
) {
  const view = useContext(ChartViewContext);
  const [width, setWidth] = useState(0);
  return {
    onLabelWidth: setWidth,
    labelBefore: labelGoesBefore(timeline, shown, view, { width, selected }),
  };
}
