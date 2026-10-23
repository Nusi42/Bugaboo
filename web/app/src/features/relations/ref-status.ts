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
