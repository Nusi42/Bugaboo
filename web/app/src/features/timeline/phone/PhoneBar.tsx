import { t } from "@bugaboo/messages";
import { part, partOf, rowStates, usePlace } from "@bugaboo/ui";
import { useRef } from "react";

import type { BarDays } from "../lines.ts";
import { hatchedStretches } from "../people/hatched-words.ts";
import { Hatches } from "../people/Hatches.tsx";
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

/** The look of the parts of the chart that the short bar of a phone draws in its narrow lane. */
const phoneLook = "phone";

/** Today in the short bar of the phone list, as the line of the chart. */
function TodayMark({ day }: { readonly day: number }) {
  const ref = useRef<HTMLSpanElement>(null);
  usePlace(ref, { "timeline-start": day });
  return <span ref={ref} {...partOf(part.timelineToday, { variants: [phoneLook] })} aria-hidden />;
}

/** The two ends of the seven weeks a bar can go on past. */
const beyond = { earlier: "earlier", later: "later" } as const;

/** The mark at an end of the seven weeks of a bar that goes on past it: *Earlier* or *Later*. */
function Beyond({ side }: { readonly side: "earlier" | "later" }) {
  return (
    <span {...partOf(part.timelinePhoneBeyond, { variants: [side] })}>
      {t(side === "earlier" ? "timeline.earlier" : "timeline.later")}
    </span>
  );
}

/**
 * The short bar of an issue in the phone list (UX §4.7, M8-Q12): seven weeks with today marked, the
 * bar on its days in the status tint and priority edge of the chart's bar, drawn by the same part on
 * the same scale (days from the first day of the span). It is a picture: the row says its dates.
 */
export function PhoneBar({ row, days, span, today }: PhoneBarProps) {
  const lane = useRef<HTMLSpanElement>(null);
  const bar = useRef<HTMLSpanElement>(null);
  // A bar that starts before the seven weeks or ends after them is cut at their edge, which says so.
  const [first, last] = [placeOf(span, days.first), placeOf(span, days.last)];
  const start = Math.max(0, first);
  const end = Math.min(span.days - 1, last);
  const stretches = hatchedStretches(row.bar?.hatched ?? [], span.first);
  usePlace(lane, { "timeline-days": span.days });
  usePlace(bar, { "timeline-start": start, "timeline-length": Math.max(0, end - start + 1) });
  const states = rowStates({
    group: row.issue.status?.status_group ?? "not-started",
    priority: row.issue.priority ?? "normal",
  });
  return (
    <span ref={lane} {...partOf(part.timelinePhoneLane)} aria-hidden>
      {end < start ? null : (
        <span ref={bar} {...partOf(part.timelineBar, { variants: [phoneLook], states })} />
      )}
      <Hatches stretches={stretches} />
      {first < 0 ? <Beyond side={beyond.earlier} /> : null}
      {last >= span.days ? <Beyond side={beyond.later} /> : null}
      {today === undefined ? null : <TodayMark day={today} />}
    </span>
  );
}
