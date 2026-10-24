import type { Schema } from "@bugaboo/api-client";
import { listRowLook, ListRow, partOf, part, useLayout } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { issueAddress } from "../../shared/paths.ts";
import { issueCells } from "../issues/index.ts";
import { statusGroupOf, statusWordsOf } from "./ref-status.ts";
import type { ShownIssue } from "./shown-issue.ts";
import { refColumns } from "./ref-columns.ts";

/** An issue whose priority the viewer may not see is drawn plain: the normal priority draws no edge. */
const plainPriority: Schema<"Priority"> = "normal";
/** An issue whose status the viewer may not see is drawn plain, like one that has not started. */
const plainGroup: Schema<"StatusGroup"> = "not-started";

export interface IssueRefRowProps {
  readonly issue: ShownIssue;
  /** What stands before the title, in muted text: *blocks*, *is caused by*. */
  readonly meaning?: string | undefined;
  /** What stands at the end of the row: a button to take the issue out or take the relation away. */
  readonly action?: ReactNode;
}

/**
 * An issue as a row of the sub-issues or the related issues (UX §4.4, R9): the status icon, the
 * issue ID, the type and the title in the same places as in the list, tinted by its status group.
 * The whole row opens the issue. What the viewer may not see of it (its status, its title) is left
 * out. The row has the edge of its priority, as in the list (M3-F173).
 */
export function IssueRefRow({ issue, meaning, action }: IssueRefRowProps) {
  const phone = useLayout() === "phone";
  const group = statusGroupOf(issue);
  const word = statusWordsOf(issue);
  return (
    <ListRow
      group={group ?? plainGroup}
      priority={issue.priority ?? plainPriority}
      columns={refColumns}
      {...(phone ? { variant: listRowLook.phone } : {})}
      cells={{
        ...issueCells(
          {
            key: issue.readable_id,
            title: issue.title ?? issue.readable_id,
            group,
            statusWord: word,
            type: issue.issue_type,
          },
          issueAddress(issue.readable_id),
          {
            before:
              meaning === undefined ? undefined : <span {...partOf(part.muted)}>{meaning} </span>,
          },
        ),
        action,
      }}
    />
  );
}
