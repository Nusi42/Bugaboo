import type { ReactNode } from "react";

import {
  AssignedToRow,
  DueDateRow,
  EstimateRows,
  LeftRow,
  PriorityRow,
  RunningTimerRow,
  SpentRows,
  StartDateRow,
  TotalRows,
} from "./ChangeableRows.tsx";
import { detailFields as fields, type RowGroup } from "./details-order.ts";
import { FixedRows } from "./FixedRows.tsx";
import { releaseGroups } from "./release-groups.tsx";
import { IssueBlockOf } from "./SlotBlocks.tsx";
import { issueBlocks, issueSlots, type IssueBlockProps } from "./slots.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

export interface DetailsGroupsProps {
  readonly blocks: IssueBlockProps;
  readonly changes: IssueChanges;
}

/** The keys of the groups that stand beside *Time spent*: the running timer and the totals. */
const [timerKey, totalsKey] = ["timer", "totals"];

/** The groups of rows of the Details: the built-in fields, then what other packages add. */
export function groupsOf({ blocks, changes }: DetailsGroupsProps) {
  const own: readonly RowGroup<ReactNode>[] = [
    {
      places: [fields.assignedTo],
      node: <AssignedToRow key={fields.assignedTo} blocks={blocks} changes={changes} />,
    },
    {
      places: [fields.priority],
      node: <PriorityRow key={fields.priority} blocks={blocks} changes={changes} />,
    },
    {
      places: [fields.reportedBy],
      node: <FixedRows key={fields.reportedBy} blocks={blocks} />,
    },
    {
      places: [fields.startDate],
      node: <StartDateRow key={fields.startDate} blocks={blocks} changes={changes} />,
    },
    {
      places: [fields.dueDate],
      node: <DueDateRow key={fields.dueDate} blocks={blocks} changes={changes} />,
    },
    {
      places: [fields.estimatedTime],
      node: <EstimateRows key={fields.estimatedTime} blocks={blocks} changes={changes} />,
    },
    {
      places: [fields.timeLeft],
      node: <LeftRow key={fields.timeLeft} blocks={blocks} changes={changes} />,
    },
    {
      places: [fields.timeSpent],
      node: <SpentRows key={fields.timeSpent} blocks={blocks} />,
    },
    {
      // A running timer shows whatever the layout says, beside *Time spent* when it is placed.
      places: [fields.timeSpent],
      node: <RunningTimerRow key={timerKey} blocks={blocks} />,
    },
    {
      places: [fields.timeSpent],
      node: <TotalRows key={totalsKey} blocks={blocks} />,
    },
  ];
  const added = issueBlocks(issueSlots.details).map((entry) => ({
    places: entry.places ?? "end",
    node: <IssueBlockOf key={entry.id} entry={entry} blocks={blocks} />,
  }));
  return [...own, ...releaseGroups({ blocks, changes }), ...added];
}
