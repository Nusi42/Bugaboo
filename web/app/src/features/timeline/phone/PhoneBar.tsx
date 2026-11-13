import { part, partOf, rowStates, usePlace } from "@bugaboo/ui";
import { useRef } from "react";

import type { BarDays } from "../lines.ts";
import { placeOf, type Span } from "../scale.ts";
import type { TimelineRow } from "../timeline-data.ts";

export interface PhoneBarProps {
  readonly row: TimelineRow;
  /** The days of the bar, as the server answered them. */
  readonly days: BarDays;
  /** The seven weeks of the list, and where today stands in them. */
  readonly span: Span;
  readonly today: number | undefined;
}

/** Today in the short bar of the phone list, as the line of the chart. */
function TodayMark({ day }: { readonly day: number }) {
  const ref = useRef<HTMLSpanElement>(null);
  usePlace(ref, { "timeline-start": day });
  return <span ref={ref} {...partOf(part.timelineToday, { variants: ["phone"] })} aria-hidden />;
}

/**
 * The short bar of an issue in the phone list (UX §4.7, M8-Q12): seven weeks with today marked, the
 * bar on its days in the status tint and priority edge of the chart's bar, drawn by the same part on
 * the same scale (days from the first day of the span). It is a picture: the row says its dates.
 */
export function PhoneBar({ row, days, span, today }: PhoneBarProps) {
  const lane = useRef<HTMLSpanElement>(null);
  const bar = useRef<HTMLSpanElement>(null);
  // A bar that starts before the seven weeks or ends after them is cut at their edge.
  const start = Math.max(0, placeOf(span, days.first));
  const end = Math.min(span.days - 1, placeOf(span, days.last));
  usePlace(lane, { "timeline-days": span.days });
  usePlace(bar, { "timeline-start": start, "timeline-length": Math.max(0, end - start + 1) });
  const states = rowStates({
    group: row.issue.status?.status_group ?? "not-started",
    priority: row.issue.priority ?? "normal",
  });
  return (
    <span ref={lane} {...partOf(part.timelinePhoneLane)} aria-hidden>
      {end < start ? null : (
        <span ref={bar} {...partOf(part.timelineBar, { variants: ["phone"], states })} />
      )}
      {today === undefined ? null : <TodayMark day={today} />}
    </span>
  );
}
