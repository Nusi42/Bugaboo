import { t } from "@bugaboo/messages";

import type { ShownIssue } from "./shown-issue.ts";

/** The words of the status of an issue where the viewer may see it: the status, or only its group. */
export function statusWordsOf(issue: ShownIssue): string | undefined {
  if (issue.status !== undefined && issue.status !== null) {
    return t(`status.${issue.status}`);
  }
  const group = issue.status_group;
  return group === undefined || group === null ? undefined : t(`status-group.${group}`);
}
