import type { ReadonlyDeep } from "@bugaboo/api-client";

import { issueAddress } from "../../shared/paths.ts";
import type { QuickResult } from "../shell/index.ts";
import { issueCells, priorityCell } from "./issue-cells.tsx";
import { lineOf, type ListedIssue } from "./issues.ts";

/** A line of the answer of the server, read only. */
export type FoundIssue = ReadonlyDeep<ListedIssue>;

/**
 * The marks of a row of the issue list for an issue that quick search found, made from the cells the
 * list draws (R9); none where the answer lacks the status, the priority or the type.
 */
export function quickIssueOf(item: FoundIssue): QuickResult["issue"] {
  const line = lineOf(item);
  if (line === undefined) {
    return undefined;
  }
  const cells = issueCells(
    {
      key: line.key,
      title: line.title,
      group: line.group,
      statusWord: line.status.name,
      type: line.issueType,
      agent: undefined,
    },
    issueAddress(line.key),
  );
  return {
    group: line.group,
    priority: line.priority,
    lead: (
      <>
        {cells.status}
        {cells.key}
        {cells.type}
      </>
    ),
    end: priorityCell(line.priority),
  };
}

/**
 * A line of the answer as a result of quick search: its ID and title, and for the look of a row of
 * the list the status, the priority and the type; nothing for a line without an ID or a title.
 */
export function resultOf(item: FoundIssue): QuickResult[] {
  const key = item.readable_id;
  const title = item.title;
  const issue = quickIssueOf(item);
  return typeof key === "string" && typeof title === "string"
    ? [{ id: key, title, issueId: key, path: issueAddress(key), ...(issue && { issue }) }]
    : [];
}
