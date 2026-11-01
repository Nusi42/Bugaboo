import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";

import { minutesText } from "../../shared/own-value-text.ts";

export { minutesText };

/** A length in a sentence: the sentence, and the length in it, which is drawn as a length. */
export interface LengthWords {
  readonly sentence: string;
  readonly length: string;
}

/** A length said with the words of `key` (`12h estimated`). */
export function lengthWords(key: MessageKey, minutes: number): LengthWords {
  const length = minutesText(minutes);
  return { sentence: t(key, { length }), length };
}

/** One total in its words; none for a total that is not there. */
function totalSaid(minutes: number | null | undefined, key: MessageKey): readonly LengthWords[] {
  return minutes === null || minutes === undefined ? [] : [lengthWords(key, minutes)];
}

/** The totals of many issues, each with its words: `12h estimated`, `9h 30m spent`; none missing. */
export function totalsWords(totals: Schema<"TimeTotalsBody">): readonly LengthWords[] {
  return [
    ...totalSaid(totals.estimated_time, "issues.total-estimated"),
    ...totalSaid(totals.time_left, "issues.total-left"),
    ...totalSaid(totals.time_spent, "issues.total-spent"),
  ];
}

/** Whether the totals have anything to say. */
export function hasTotals(totals: Schema<"TimeTotalsBody"> | null | undefined): boolean {
  return totals !== null && totals !== undefined && totalsWords(totals).length > 0;
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
