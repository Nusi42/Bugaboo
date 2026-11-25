import { t } from "@bugaboo/messages";
import { part, partOf, usePlace, type PartAttributes } from "@bugaboo/ui";
import { useRef, type ReactNode } from "react";

import type { TimelineRelease } from "./drivers.ts";
import { placeOf, type Span } from "./scale.ts";
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

/** Whether a day stands on the chart. */
function onChart(span: Span, day: string): boolean {
  const at = placeOf(span, day);
  return at >= 0 && at < span.days;
}

/**
 * The releases of the page with a *Release date* on the chart (ADR-0043, amendment of 2026-11-24): a
 * line through the day with the diamond and the name of the release at the top. The rows of a release
 * say its name in words; the line is a picture.
 */
export function ReleaseLines({ releases }: { readonly releases: readonly TimelineRelease[] }) {
  const { span } = useTimeline();
  return (
    <>
      {releases.map((release) =>
        release.release_date === null || !onChart(span, release.release_date) ? null : (
          <Placed
            key={release.id}
            look={partOf(part.timelineRelease)}
            start={placeOf(span, release.release_date)}
          >
            <span {...partOf(part.timelineReleaseLabel)}>
              <span {...partOf(part.timelineReleaseDiamond)} />
              {release.name}
            </span>
          </Placed>
        ),
      )}
    </>
  );
}

/** The diamond of a release on its *Release date*, in the head of its group (*Group by release*). */
export function ReleaseDiamond({ release }: { readonly release: TimelineRelease }) {
  const { span } = useTimeline();
  return release.release_date === null ? null : (
    <Placed look={partOf(part.timelineDiamond)} start={placeOf(span, release.release_date)} />
  );
}
