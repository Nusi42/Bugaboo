import type { Schema } from "@bugaboo/api-client";

import type { ShownIssue } from "./shown-issue.ts";

/** The words of the status of an issue where the viewer may see it. */
export function statusWordsOf(issue: ShownIssue): string | undefined {
  return issue.status?.name;
}

/** The status group of an issue where the viewer may see its status. */
export function statusGroupOf(issue: ShownIssue): Schema<"StatusGroup"> | undefined {
  return issue.status?.status_group;
}

/** An issue whose status the viewer may not see is drawn plain, like one that has not started. */
const plainGroup: Schema<"StatusGroup"> = "not-started";

/** An issue whose priority the viewer may not see is drawn plain: the normal priority draws no edge. */
const plainPriority: Schema<"Priority"> = "normal";

/** The status group a row or a card of an issue is tinted by: its own, or plain where it is hidden. */
export function drawnGroupOf(issue: ShownIssue): Schema<"StatusGroup"> {
  return statusGroupOf(issue) ?? plainGroup;
}

/** The priority a row or a card of an issue is edged by: its own, or plain where it is hidden. */
export function drawnPriorityOf(issue: ShownIssue): Schema<"Priority"> {
  return issue.priority ?? plainPriority;
}
