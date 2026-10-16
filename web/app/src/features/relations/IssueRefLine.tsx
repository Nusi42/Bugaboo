import { StatusPill, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { statusWordsOf } from "./ref-status.ts";
import type { ShownIssue } from "./shown-issue.ts";

/**
 * One line for an issue: its readable ID, its title and, where the viewer may see it, its status
 * as the pill of its group. What is not in the answer is not drawn, and nothing says that
 * something was left out. `children` follow the status (a button to take it out).
 */
export function IssueRefLine({
  issue,
  children,
}: {
  readonly issue: ShownIssue;
  readonly children?: ReactNode;
}) {
  const group = issue.status_group;
  const word = statusWordsOf(issue);
  return (
    <span {...partOf(part.issueRef)}>
      <strong {...partOf(part.strong)}>{issue.key}</strong>
      {issue.title === undefined || issue.title === null ? null : <span>{issue.title}</span>}
      {group === undefined || group === null || word === undefined ? null : (
        <StatusPill group={group} label={word} />
      )}
      {children}
    </span>
  );
}
