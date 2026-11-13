import { textByKey } from "@bugaboo/messages";

import { formatCalendarDay, keepingHyphens } from "../../../shared/format.ts";
import { minutesText } from "../../../shared/own-value-text.ts";
import { personName } from "../../../shared/person.ts";
import { daysText } from "../dates.ts";
import type { PlanWarning } from "../timeline-data.ts";

/** What a sentence of a warning names besides what the warning holds: the issue and its project. */
export interface WarningPlace {
  /** The readable ID of the issue the warning is on. */
  readonly key: string;
  /** The name of its project, for *Too much at once*. */
  readonly project: string;
}

/** The days of a warning: one day, or the first and the last. */
function warningDays(warning: PlanWarning): string {
  return warning.days.first === warning.days.last
    ? formatCalendarDay(warning.days.first)
    : daysText(warning.days);
}

/**
 * The sentence of a warning (UX §4.7, PLAN-13): the server picks the message (`sentence`) and sends
 * what it holds; the screen only fills in the words of each member, the way the issue page fills in
 * any message of the server. Nothing here judges the plan.
 */
export function warningSentence(warning: PlanWarning, place: WarningPlace): string {
  // An Issue ID never breaks at its hyphen (WEB- / 280).
  const other = keepingHyphens(warning.other?.readable_id ?? "");
  const days = warning.late_days ?? 0;
  return textByKey(warning.sentence, {
    issue: keepingHyphens(place.key),
    days: warning.kind === "delay" ? days : warningDays(warning),
    person: warning.person === null ? "" : personName(warning.person),
    first: other,
    sub_issue: other,
    needs: warning.needs_days ?? 0,
    has: warning.has_days ?? 0,
    planned: minutesText(warning.planned_minutes ?? 0),
    can: minutesText(warning.can_minutes ?? 0),
    day: warning.worst_day === null ? "" : formatCalendarDay(warning.worst_day),
    project: place.project,
  });
}
