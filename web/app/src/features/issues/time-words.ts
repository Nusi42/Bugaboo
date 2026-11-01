import { secondsPerMinute, type Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";

import { detailsText } from "../../shared/list-text.ts";
import { durationText } from "../../shared/own-value-text.ts";

/** A length in whole minutes in the short form of a row and a column (`1h 30m`, UX §4.9). */
export function minutesText(minutes: number): string {
  return durationText(minutes * secondsPerMinute);
}

/** One total in words (`12h estimated`); none for a total that is not there. */
function totalSaid(minutes: number | null | undefined, key: MessageKey): readonly string[] {
  return minutes === null || minutes === undefined
    ? []
    : [t(key, { length: minutesText(minutes) })];
}

/** The totals of many issues in words: `12h estimated · 9h 30m spent`; empty when there are none. */
export function totalsText(totals: Schema<"TimeTotalsBody">): string {
  return detailsText([
    ...totalSaid(totals.estimated_time, "issues.total-estimated"),
    ...totalSaid(totals.time_left, "issues.total-left"),
    ...totalSaid(totals.time_spent, "issues.total-spent"),
  ]);
}

/** *Time left*: the minutes, and whether they are worked out rather than typed. */
export interface TimeLeft {
  readonly minutes: number;
  readonly worked_out: boolean;
}

/** The *Time left* of an issue as an answer holds it; none while it has none. */
export function timeLeftOf(
  issue: Pick<Schema<"IssueItem">, "time_left" | "time_left_worked_out">,
): TimeLeft | undefined {
  return issue.time_left === undefined || issue.time_left === null
    ? undefined
    : { minutes: issue.time_left, worked_out: issue.time_left_worked_out === true };
}

/** *Time left* as a cell or a row reads it: the minutes, and whether they are worked out. */
export function timeLeftText(left: TimeLeft): string {
  return left.worked_out
    ? t("issues.time-left-worked-out-short", { length: minutesText(left.minutes) })
    : minutesText(left.minutes);
}

/** A range of lengths that a list is grouped in, in words: `Under 1h`, `1h to 4h`, `40h or more`. */
export function rangeText(range: Schema<"SearchLengthRangeBody">): string {
  const to = range.to ?? undefined;
  if (range.from === 0 && to !== undefined) {
    return t("issues.length-range-under", { to: minutesText(to) });
  }
  return to === undefined
    ? t("issues.length-range-over", { from: minutesText(range.from) })
    : t("issues.length-range", { from: minutesText(range.from), to: minutesText(to) });
}
