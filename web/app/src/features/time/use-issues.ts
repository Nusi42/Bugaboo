import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { choicesHeldIn, useFieldName, useFields } from "../fields/index.ts";
import { issueRows, type IssueRow } from "./rows.ts";
import type { IssuesProps } from "./sheet-props.ts";
import { useBookedWeek, useStarters, useWeekEntries } from "./use-week.ts";

/** The name of a field, or the word of the glossary while the name is not known (it comes back as the ID). */
function nameOr(name: string, id: string | undefined, word: string): string {
  return id === undefined || name === id ? word : name;
}

/** Reads the issue the person picked: the issue, or the sentence of why it could not be read. */
async function readIssue(key: string) {
  try {
    const found = dataOf(
      await api.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
    );
    return { issue: found.issue, problem: undefined };
  } catch (error) {
    return { issue: undefined, problem: messageOf(error) };
  }
}

/** The issues the person added on the page, which have no time in the week yet: their rows are empty. */
function useAddedIssues() {
  const [added, setAdded] = useState<readonly IssueRow[]>([]);
  const [problem, setProblem] = useState<string | undefined>();
  const add = async (key: string): Promise<void> => {
    const found = await readIssue(key);
    if (found.problem !== undefined) {
      setProblem(found.problem);
      return;
    }
    setProblem(undefined);
    if (found.issue?.id === undefined || found.issue.id === null) {
      return;
    }
    const row: IssueRow = {
      issue: found.issue.id,
      readableId: found.issue.readable_id ?? key,
      title: found.issue.title ?? "",
      activity: [],
      costCenter: [],
      choiceMinutes: {},
      entries: {},
    };
    setAdded((was) => [...was.filter((each) => each.issue !== row.issue), row]);
  };
  return { added, add, problem };
}

/**
 * The time on issues of one's own week: a row for each issue with entries in it and each issue
 * added on the page, and the comparison with the working hours as the
 * server works it out.
 */
export function useIssues(
  first: string,
  onDetails: (issue: string) => void,
): {
  readonly issues: IssuesProps;
  readonly ready: boolean;
} {
  const entries = useWeekEntries(first);
  const starters = useStarters();
  const booked = useBookedWeek(first);
  const { added, add, problem } = useAddedIssues();
  // Retired fields too: the entries that were booked with them keep their choices on the sheet.
  const fields = useFields({ retired: true }).data?.fields ?? [];
  const fromEntries = issueRows(entries.entries, starters, choicesHeldIn(fields));
  const rows = [
    ...fromEntries,
    ...added.filter((row) => !fromEntries.some((each) => each.issue === row.issue)),
  ];
  const activity = useFieldName(undefined, starters.activity ?? "");
  const costCenter = useFieldName(undefined, starters.costCenter ?? "");
  const week = booked.data?.week;
  const days = Object.fromEntries(
    (booked.data?.days ?? []).map((day) => [
      day.date,
      { booked: day.booked, notBooked: day.not_booked },
    ]),
  );
  return {
    ready: entries.complete && booked.data !== undefined,
    issues: {
      rows,
      activityName: nameOr(activity, starters.activity, t("time.sheet.activity")),
      costCenterName: nameOr(costCenter, starters.costCenter, t("time.sheet.cost-center")),
      booked: days,
      week: week === undefined ? undefined : { booked: week.booked, notBooked: week.not_booked },
      onDetails,
      addProblem: problem,
      onAdd: (key) => {
        void add(key);
      },
    },
  };
}
