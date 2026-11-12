import { t } from "@bugaboo/messages";
import { part, partOf, usePlace, type PartAttributes } from "@bugaboo/ui";
import { useRef, type ReactNode } from "react";

import { placeOf } from "./scale.ts";
import { useTimeline } from "./timeline-context.ts";
import type { TimelineGoal } from "./timeline-data.ts";

interface PlacedProps {
  readonly look: PartAttributes;
  /** The first day it covers, counted from the first day of the chart, and how many days. */
  readonly start: number;
  readonly length?: number;
  readonly children?: ReactNode;
}

/** Something that stands on days of the chart: a weekend, the Today line, a bracket, a diamond. */
function Placed({ look, start, length = 1, children }: PlacedProps) {
  const ref = useRef<HTMLSpanElement>(null);
  usePlace(ref, { "timeline-start": start, "timeline-length": length });
  return (
    <span ref={ref} {...look} aria-hidden>
      {children}
    </span>
  );
}

/** A weekend, shaded from the scale down behind the bars. */
export function Shade({ start, length }: { readonly start: number; readonly length: number }) {
  return <Placed look={partOf(part.timelineShade)} start={start} length={length} />;
}

/** The Today line in the accent color, with its word at the top (UX §4.7). */
export function TodayLine({ day }: { readonly day: number }) {
  return (
    <Placed look={partOf(part.timelineToday)} start={day}>
      <span {...partOf(part.timelineTodayLabel)}>{t("timeline.today")}</span>
    </Placed>
  );
}

/**
 * A goal on the timeline (ADR-0043, decision 7): a bracket over the days of its sub-issues, and a
 * diamond on its due date. Both come from the server; neither is worked out here.
 */
export function GoalMarks({ goal }: { readonly goal: TimelineGoal }) {
  const { span } = useTimeline();
  const bracket = goal.span ?? undefined;
  const start = bracket === undefined ? 0 : placeOf(span, bracket.first);
  return (
    <>
      {bracket === undefined ? null : (
        <Placed
          look={partOf(part.timelineBracket)}
          start={start}
          length={placeOf(span, bracket.last) - start + 1}
        />
      )}
      {goal.due === null ? null : (
        <Placed look={partOf(part.timelineDiamond)} start={placeOf(span, goal.due)} />
      )}
    </>
  );
}
