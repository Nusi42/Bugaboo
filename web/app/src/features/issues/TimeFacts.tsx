import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Facts, Length, NameValueRow, partOf, part, SentenceWithValue } from "@bugaboo/ui";

import { keepingHyphens } from "../../shared/format.ts";
import { AddTimeButton, TimerButton, TimerNow, useTimerOn } from "../time/index.ts";
import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { entryIssueOf } from "./entry-issue.ts";

import { minutesText, totalsWords, type LengthWords, type TimeLeft } from "./time-words.ts";

/** A sentence with a length in it: the length in the code font, the words around it as they are. */
export function LengthIn({ words }: { readonly words: LengthWords }) {
  return (
    <SentenceWithValue sentence={words.sentence} value={words.length}>
      {(length) => <Length text={length} />}
    </SentenceWithValue>
  );
}

/**
 * The totals of many issues side by side (*10h estimated · 6h 40m left · 3h 20m spent*): each in
 * its words with its length in the code font, breaking only between two of them.
 */
export function TotalFacts({ totals }: { readonly totals: Schema<"TimeTotalsBody"> }) {
  return (
    <Facts
      items={totalsWords(totals).map((words) => ({
        key: words.sentence,
        node: <LengthIn words={words} />,
      }))}
    />
  );
}

/** A length of whole minutes, as a row or a cell shows it. */
export function MinutesLength({ minutes }: { readonly minutes: number }) {
  return <Length text={minutesText(minutes)} />;
}

/** *Time left* as a cell shows it: the length, in the quiet style of a value worked out when it is. */
export function TimeLeftLength({ left }: { readonly left: TimeLeft }) {
  const length = <Length text={minutesText(left.minutes)} />;
  return left.worked_out ? <span {...partOf(part.hint)}>{length}</span> : length;
}

/** The row of the totals with the sub-issues: its name breaks at its space, never at its hyphen. */
export function WithSubIssuesRow({ totals }: { readonly totals: Schema<"TimeTotalsBody"> }) {
  return (
    <NameValueRow name={keepingHyphens(t("issues.with-sub-issues"))}>
      <TotalFacts totals={totals} />
    </NameValueRow>
  );
}

/** *Your timer*: how long it has run and *Stop*, in a row of the Details while the timer runs on this issue. */
export function TimerRow({ blocks }: Pick<ChangeableRowsProps, "blocks">) {
  const issue = entryIssueOf(blocks.issue);
  const { state } = useTimerOn(issue?.key ?? "");
  if (issue === undefined || state !== "here") {
    return null;
  }
  return (
    <DetailRow name={t("time.timer.row")}>
      <TimerNow />
    </DetailRow>
  );
}

/**
 * *Time spent*: the length that is read and never typed, and *Add time* beside it for a person who
 * may book time on the issue.
 */
export function SpentRow({
  blocks,
  minutes,
}: Pick<ChangeableRowsProps, "blocks"> & { readonly minutes: number }) {
  const issue = entryIssueOf(blocks.issue);
  return (
    <DetailRow name={t("issue-field.time-spent")}>
      <MinutesLength minutes={minutes} />
      {issue === undefined ? null : (
        <>
          <TimerButton issue={issue} />
          <AddTimeButton issue={issue} />
        </>
      )}
    </DetailRow>
  );
}
