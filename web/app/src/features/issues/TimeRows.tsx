import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { NameValueRow, partOf, part } from "@bugaboo/ui";

import { LengthBox } from "../fields/index.ts";
import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { rowStatus } from "./row-status.ts";
import { LengthIn, MinutesLength, TotalFacts } from "./TimeFacts.tsx";
import { hasTotals, lengthWords, timeLeftOf } from "./time-words.ts";
import { useIssueTime } from "./use-issue-time.ts";

/** The look of a name that keeps its hyphen on one line. */
const unbroken = "whole";

/** *Estimated time*: a box for who may change it, the length for who may only read it. */
function EstimateRow({ blocks, changes }: ChangeableRowsProps) {
  const { issue, mask } = blocks;
  const name = t("issue-field.estimated-time");
  const minutes = issue.estimated_time ?? undefined;
  if (!mask.sees("estimated-time")) {
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
 * *Time left*: the box holds what somebody typed; a value nobody typed is worked out from the
 * estimate and the time spent and said quietly beside it, and typing one makes it count until it is
 * cleared.
 */
function LeftRow({
  blocks,
  changes,
  time,
}: ChangeableRowsProps & { readonly time: Schema<"IssueTimeBody"> | undefined }) {
  const { issue, mask } = blocks;
  const name = t("issue-field.time-left");
  const typed = timeLeftOf(issue);
  const typedMinutes = typed === undefined || typed.worked_out ? undefined : typed.minutes;
  const left = time?.time_left ?? undefined;
  const workedOut = left?.worked_out === true ? left.minutes : undefined;
  if (!mask.sees("time-left")) {
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
 * The lengths of time of the issue itself in the Details (TIME-03, ADR-0035 decision 4):
 * *Estimated time* and *Time left*, which change in place like any field, *Time spent*, which is
 * read and never typed, and the totals with the sub-issues. Each follows its own level. They are
 * fields of the issue and stand among its rows, apart from the defaults for time booked on it.
 */
export function TimeRows({ blocks, changes }: ChangeableRowsProps) {
  const time = useIssueTime(blocks.readableId, blocks.version).data ?? undefined;
  const spent = time?.time_spent ?? undefined;
  const totals = time?.with_sub_issues;
  return (
    <>
      <EstimateRow blocks={blocks} changes={changes} />
      <LeftRow blocks={blocks} changes={changes} time={time} />
      {spent === undefined ? null : (
        <DetailRow name={t("issue-field.time-spent")}>
          <MinutesLength minutes={spent} />
        </DetailRow>
      )}
      {totals === undefined || totals === null || !hasTotals(totals) ? null : (
        <NameValueRow name={t("issues.with-sub-issues")} nameLook={unbroken}>
          <TotalFacts totals={totals} />
        </NameValueRow>
      )}
    </>
  );
}
