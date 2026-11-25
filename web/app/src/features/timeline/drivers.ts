import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { detailsText } from "../../shared/list-text.ts";
import type { TimelineGoal } from "./timeline-data.ts";

/**
 * Deadline drivers and releases as the server answers them (PLAN-05, ADR-0043 amendment of
 * 2026-11-24): which ends an issue of the page sets, and the releases of the page. Nothing here
 * works out a driver; the screen only names what the server sent.
 */

/** An end a deadline driver sets: a goal, a release, or the last day of the view. */
export type TimelineEnd = Schema<"TimelineEndBody">;

/** A release of the page, with its *Release date*. */
export type TimelineRelease = Schema<"TimelineReleaseBody">;

/** What the names of the ends are looked up in. */
export interface EndNames {
  readonly goalOf: (issue: string) => TimelineGoal | undefined;
  readonly releaseOf: (release: string) => TimelineRelease | undefined;
}

/** The name of an end: the title of a goal, the name of a release, or *the view*. */
function endName(end: TimelineEnd, names: EndNames): string {
  const id = end.id ?? "";
  if (end.kind === "goal") {
    return names.goalOf(id)?.title ?? t("timeline.end-goal");
  }
  if (end.kind === "release") {
    return t("timeline.end-release", { name: names.releaseOf(id)?.name ?? "" });
  }
  return t("timeline.end-view");
}

/**
 * What a driver sets, in words for its card and for screen readers: *"Sets the end of Mobile app
 * 2.0"*, one sentence for each end; none for an issue that sets no end.
 */
export function setsText(ends: readonly TimelineEnd[], names: EndNames): string | undefined {
  return ends.length === 0
    ? undefined
    : detailsText(ends.map((end) => t("timeline.sets-end", { end: endName(end, names) })));
}
