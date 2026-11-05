import type { EntryIssue } from "../time/index.ts";
import type { IssueView } from "./slots.ts";

/** The issue as the forms and buttons of time need it; none while the page does not know its ID or project. */
export function entryIssueOf(issue: IssueView): EntryIssue | undefined {
  const project = issue.project?.short_name;
  const key = issue.readable_id;
  return project === undefined || key === undefined || key === null
    ? undefined
    : { key, title: issue.title ?? "", project, issueType: issue.issue_type?.id };
}
