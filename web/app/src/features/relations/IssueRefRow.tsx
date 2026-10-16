import type { Schema } from "@bugaboo/api-client";
import {
  IssueTypeIcon,
  Link,
  listRowLook,
  ListRow,
  StatusIcon,
  iconSize,
  partOf,
  part,
  useLayout,
} from "@bugaboo/ui";
import type { ReactNode } from "react";

import { issueAddress } from "../../shared/paths.ts";
import { statusWordsOf } from "./ref-status.ts";
import type { ShownIssue } from "./shown-issue.ts";
import { refColumns } from "./ref-columns.ts";

/** An issue reference has no priority, so its row shows none: the normal one draws no edge. */
const noPriority: Schema<"Priority"> = "normal";
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
 * out. The priority is not part of an issue reference, so no edge is drawn.
 */
export function IssueRefRow({ issue, meaning, action }: IssueRefRowProps) {
  const phone = useLayout() === "phone";
  const group = issue.status_group;
  const word = statusWordsOf(issue);
  return (
    <ListRow
      group={group ?? plainGroup}
      priority={noPriority}
      columns={refColumns}
      {...(phone ? { variant: listRowLook.phone } : {})}
      cells={{
        status:
          group === undefined || group === null || word === undefined ? undefined : (
            <StatusIcon group={group} size={iconSize.small} label={word} />
          ),
        key: issue.key,
        type: <IssueTypeIcon name={issue.issue_type.icon} label={issue.issue_type.name} />,
        title: (
          <>
            {meaning === undefined ? null : <span {...partOf(part.muted)}>{meaning} </span>}
            <Link href={issueAddress(issue.key)}>{issue.title ?? issue.key}</Link>
          </>
        ),
        action,
      }}
    />
  );
}
