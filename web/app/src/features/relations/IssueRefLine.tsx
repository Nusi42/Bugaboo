import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

/** An issue as the server shows it in a list: what the viewer may see of it in its own project. */
export type ShownIssue = Schema<"IssueRef">;

const statusMessage = {
  "not-started": "issue-ref.status-not-started",
  "in-progress": "issue-ref.status-in-progress",
  done: "issue-ref.status-done",
} as const;

/**
 * One line for an issue: its readable ID, its title and, where the viewer may see it, its status
 * in the color of its group. What is not in the answer is not drawn, and nothing says that
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
  return (
    <span {...partOf(part.issueRef)}>
      <strong {...partOf(part.strong)}>{issue.key}</strong>
      {issue.title === undefined || issue.title === null ? null : <span>{issue.title}</span>}
      {group === undefined || group === null ? null : (
        <span {...partOf(part.issueRefStatus, { variants: [group] })}>
          {t(statusMessage[group])}
        </span>
      )}
      {children}
    </span>
  );
}
