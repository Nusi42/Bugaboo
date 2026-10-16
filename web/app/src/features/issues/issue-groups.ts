import { statusGroupWords } from "@bugaboo/api-client";
import type { StatusGroup } from "@bugaboo/ui";

import type { IssueLine } from "./issues.ts";

/**
 * Where each group of statuses stands in a list grouped by status (UX §4.2): what is being worked
 * on first, then what waits, then what is done. A group the server knows and this table lacks is a
 * type error.
 */
const place = {
  "in-progress": 0,
  "not-started": 1,
  done: 2,
} as const satisfies Readonly<Record<StatusGroup, number>>;

/** The issues of one group of statuses, in the order the list had them. */
export interface IssueGroup {
  readonly group: StatusGroup;
  readonly lines: readonly IssueLine[];
}

/** The groups in the order a list shows them. */
const groupsInOrder: readonly StatusGroup[] = statusGroupWords.toSorted(
  (first, second) => place[first] - place[second],
);

/** The lines grouped by status group in the order of the groups; a group without issues is left out. */
export function groupByStatus(lines: readonly IssueLine[]): readonly IssueGroup[] {
  return groupsInOrder
    .map((group): IssueGroup => ({ group, lines: lines.filter((line) => line.group === group) }))
    .filter((each) => each.lines.length > 0);
}
