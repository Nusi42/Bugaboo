import { t } from "@bugaboo/messages";

import { formatCalendarDay } from "../../shared/format.ts";
import { daysText } from "./dates.ts";
import type { TimelineRelease } from "./drivers.ts";
import type { TimelineGoal } from "./timeline-data.ts";

/**
 * A goal in words, for screen readers (the bracket and the diamond only draw it): the days of its
 * sub-issues and its due date, where it has them.
 */
export function goalWords(goal: TimelineGoal, key: string): string {
  return [
    goal.span === null ? undefined : t("timeline.goal", { key, days: daysText(goal.span) }),
    goal.due === null
      ? undefined
      : t("timeline.goal-due", { key, day: formatCalendarDay(goal.due) }),
  ]
    .filter((words) => words !== undefined)
    .join(" ");
}

/** The *Release date* of a release in words, *"Release date: 10 Mar 2027"*; empty without one. */
export function releaseWords(release: TimelineRelease): string {
  return release.release_date === null
    ? ""
    : t("timeline.release-date", { day: formatCalendarDay(release.release_date) });
}
