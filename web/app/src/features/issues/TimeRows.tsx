import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { LengthBox } from "../fields/index.ts";
import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { rowStatus } from "./row-status.ts";
import { minutesText, timeLeftOf, totalsText } from "./time-words.ts";

/** The totals of the issue with its sub-issues, read when the row is drawn. */
function useWithSubIssues(issue: string, version: number) {
  return useQuery({
    queryKey: queryKeys.issues.time(issue, version),
    queryFn: async () =>
      (await api.client.GET("/api/v1/issues/{issue}/time", { params: { path: { issue } } })).data,
  });
}

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
        minutesText(minutes)
      )}
    </DetailRow>
  );
}

/**
 * *Time left*: the box holds what somebody typed; a value nobody typed is worked out from the
 * estimate and the time spent and said quietly beside it, and typing one makes it count until it is
 * cleared.
 */
function LeftRow({ blocks, changes }: ChangeableRowsProps) {
  const { issue, mask } = blocks;
  const name = t("issue-field.time-left");
  const left = timeLeftOf(issue);
  const typed = left === undefined || left.worked_out ? undefined : left.minutes;
  const workedOut = left?.worked_out === true ? minutesText(left.minutes) : undefined;
  if (!mask.sees("time-left")) {
    return null;
  }
  const changeable = mask.changes("time-left");
  return (
    <DetailRow name={name} status={rowStatus(changes, "time-left")}>
      {changeable ? (
        <LengthBox
          name={name}
          minutes={typed}
          onCommit={(minutes) => {
            changes.change({ timeLeft: minutes });
          }}
        />
      ) : typed === undefined ? null : (
        minutesText(typed)
      )}
      {workedOut === undefined ? null : (
        <span {...partOf(part.hint)}>
          {t(changeable ? "issues.time-left-worked-out" : "issues.time-left-worked-out-short", {
            length: workedOut,
          })}
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
  const spent = blocks.issue.time_spent ?? undefined;
  const totals = useWithSubIssues(blocks.readableId, blocks.version).data?.with_sub_issues;
  const totalWords = totals === undefined || totals === null ? "" : totalsText(totals);
  return (
    <>
      <EstimateRow blocks={blocks} changes={changes} />
      <LeftRow blocks={blocks} changes={changes} />
      {spent === undefined ? null : (
        <DetailRow name={t("issue-field.time-spent")}>{minutesText(spent)}</DetailRow>
      )}
      {totalWords === "" ? null : (
        <DetailRow name={t("issues.with-sub-issues")}>{totalWords}</DetailRow>
      )}
    </>
  );
}
