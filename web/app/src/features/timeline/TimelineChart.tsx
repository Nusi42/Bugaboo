import { t } from "@bugaboo/messages";
import { part, partOf, pixelsOf } from "@bugaboo/ui";
import { useId } from "react";

import { ChartBackdrop, type ChartBackdropProps } from "./ChartBackdrop.tsx";
import type { GroupLine, Line } from "./lines.ts";
import { useCanvas } from "./use-canvas.ts";
import { TimelineLine } from "./TimelineLine.tsx";
import { useDayInView, useVisibleRows } from "./use-visible-rows.ts";

export interface TimelineChartProps extends ChartBackdropProps {
  readonly lines: readonly Line[];
  readonly groupName: (line: GroupLine) => string;
  readonly onToggle: (group: string) => void;
}

/**
 * The chart of the timeline (UX §4.7): one box that scrolls both ways, with the scale at its top,
 * the names of the rows at its start, and only the rows that are on screen drawn.
 */
export function TimelineChart({ lines, groupName, onToggle, ...backdrop }: TimelineChartProps) {
  const { canvas, drop, rowPixels, dayPixels } = useCanvas(lines.length);
  const { box, visible } = useVisibleRows(lines.length, rowPixels, pixelsOf("size-timeline-head"));
  useDayInView(box, backdrop.today === undefined ? undefined : backdrop.today * dayPixels);
  const described = { keys: useId(), locked: useId() };
  return (
    <div ref={box} {...partOf(part.timelineScroll)}>
      <div
        ref={canvas}
        {...partOf(part.timelineCanvas, { states: [drop.isDropTarget && "over"] })}
        {...drop.dropProps}
      >
        <ChartBackdrop {...backdrop} />
        <ul {...partOf(part.timelineRows)} aria-label={t("timeline.chart")}>
          {lines.slice(visible.first, visible.last + 1).map((line, index) => (
            <TimelineLine
              key={line.id}
              line={line}
              at={visible.first + index}
              described={described}
              groupName={groupName}
              onToggle={onToggle}
            />
          ))}
        </ul>
      </div>
      <p id={described.keys} {...partOf(part.visuallyHidden)}>
        {t("timeline.keys")}
      </p>
      <p id={described.locked} {...partOf(part.visuallyHidden)}>
        {t("timeline.locked-here")}
      </p>
    </div>
  );
}
