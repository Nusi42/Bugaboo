import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { LengthBox } from "../fields/index.ts";
import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { rowStatus } from "./row-status.ts";
import { LengthIn, MinutesLength, SpentRow, WithSubIssuesRow } from "./TimeFacts.tsx";
import { TimeFactorRow } from "./TimeFactorRow.tsx";
import { hasTotals, lengthWords, timeLeftOf } from "./time-words.ts";
import { useIssueTime } from "./use-issue-time.ts";

/** *Estimated time*: a box for who may change it, the length for who may only read it. */
export function EstimateRow({ blocks, changes }: ChangeableRowsProps) {
  const { issue, mask, form } = blocks;
  const name = t("issue-field.estimated-time");
  const minutes = issue.estimated_time ?? undefined;
  if (!form.shows("estimated-time") || !mask.sees("estimated-time")) {
    return null;
  }
  return (
    <DetailRow name={name} status={rowStatus(changes, "estimated-time")}>
      {mask.changes("estimated-time") ? (
        <LengthBox
          name={name}
          minutes={minutes}
          onCommit={(typed) => {
            changes.change({ estimatedTime: typed });
          }}
        />
      ) : minutes === undefined ? null : (
        <MinutesLength minutes={minutes} />
      )}
    </DetailRow>
  );
}

/**
 * *Estimated time* and, under it, *Time factor* (UX §4.4): the factor is no field of a layout, so it
 * stands where the estimate stands. It shows also where the estimate is hidden, for the person on the
 * bar, who always sees their own factor.
 */
export function EstimateRows({ blocks, changes }: ChangeableRowsProps) {
  return (
    <>
      <EstimateRow blocks={blocks} changes={changes} />
      <TimeFactorRow blocks={blocks} />
    </>
  );
}

/**
 * *Time left*: the box holds what somebody typed; a value nobody typed is worked out from the
 * estimate and the time spent and said quietly beside it, and typing one makes it count until it is
 * cleared.
 */
export function LeftRow({ blocks, changes }: ChangeableRowsProps) {
  const { issue, mask, form } = blocks;
  const time = useIssueTime(blocks.readableId).data ?? undefined;
  const name = t("issue-field.time-left");
  const typed = timeLeftOf(issue);
  const typedMinutes = typed === undefined || typed.worked_out ? undefined : typed.minutes;
  const left = time?.time_left ?? undefined;
  const workedOut = left?.worked_out === true ? left.minutes : undefined;
  // A worked-out *Time left* is no member of the issue: the time of the issue holds it.
  if (!form.shows("time-left") || !(mask.sees("time-left") || left !== undefined)) {
    return null;
  }
  const changeable = mask.changes("time-left");
  return (
    <DetailRow name={name} status={rowStatus(changes, "time-left")}>
      {changeable ? (
        <LengthBox
          name={name}
          minutes={typedMinutes}
          onCommit={(minutes) => {
            changes.change({ timeLeft: minutes });
          }}
        />
      ) : typedMinutes === undefined ? null : (
        <MinutesLength minutes={typedMinutes} />
      )}
      {workedOut === undefined ? null : (
        <span {...partOf(part.hint)}>
          <LengthIn
            words={lengthWords(
              changeable ? "issues.time-left-worked-out" : "issues.time-left-worked-out-short",
              workedOut,
            )}
          />
        </span>
      )}
    </DetailRow>
  );
}

/**
 * *Time spent*, which is read and never typed, with *Add time* and the timer button beside it
 * (TIME-03, ADR-0035 decision 4). It follows its own level and stands where the layout puts
 * *Time spent*.
 */
export function SpentRows({ blocks }: Pick<ChangeableRowsProps, "blocks">) {
  const spent = useIssueTime(blocks.readableId).data?.time_spent ?? undefined;
  return spent === undefined || !blocks.form.shows("time-spent") ? null : (
    <SpentRow blocks={blocks} minutes={spent} />
  );
}

/**
 * The totals with the sub-issues, below *Time spent* and shown with it: they add up the three
 * lengths of the sub-issues the person may read.
 */
export function TotalRows({ blocks }: Pick<ChangeableRowsProps, "blocks">) {
  const totals = useIssueTime(blocks.readableId).data?.with_sub_issues;
  return totals === undefined ||
    totals === null ||
    !hasTotals(totals) ||
    !blocks.form.shows("time-spent") ? null : (
    <WithSubIssuesRow totals={totals} />
  );
}

/**
 * *Your timer* while it runs on this issue. It shows wherever the layout puts the lengths of time,
 * and when it places none: a timer that runs is never hidden. It stands beside *Time spent* when
 * the layout places it, else last.
 */
export { TimerRow as RunningTimerRow } from "./TimeFacts.tsx";
