import { t } from "@bugaboo/messages";
import { ListGrid, ListGroup, ListHead, useLayout } from "@bugaboo/ui";
import { Fragment } from "react";

import { useNow } from "../../shared/use-now.ts";
import { hrefOf, type IssueShow } from "./issue-show.ts";
import { groupByStatus } from "./issue-groups.ts";
import { IssueRow } from "./IssueRow.tsx";
import type { IssueLine } from "./issues.ts";
import { issueColumns, listColumnLabels } from "./list-labels.ts";
import type { IssueSelection } from "./use-selection.ts";

export interface IssueGridProps {
  readonly lines: readonly IssueLine[];
  /** The short name of the project. */
  readonly project: string;
  readonly openIn: IssueShow;
  /** The issue that is open next to the list, by its issue ID. */
  readonly open: string | undefined;
  readonly selection: IssueSelection;
}

/**
 * The grid of the issue list (UX §4.2): the names of the columns, then the rows under the header
 * of their status group (*Group by status*). A phone has no header and draws each row in two lines.
 */
export function IssueGrid({ lines, project, openIn, open, selection }: IssueGridProps) {
  const phone = useLayout() === "phone";
  const now = useNow();
  return (
    <ListGrid label={t("issues.list")}>
      {phone ? null : <ListHead labels={listColumnLabels()} columns={issueColumns} />}
      {groupByStatus(lines).map(({ group, lines: inGroup }) => (
        <Fragment key={group}>
          <ListGroup group={group} label={t(`status-group.${group}`)} count={inGroup.length} />
          {inGroup.map((line) => (
            <IssueRow
              key={line.id}
              line={line}
              href={hrefOf(openIn, project, line.key)}
              selected={selection.has(line.id)}
              current={line.key === open}
              phone={phone}
              now={now}
              onSelect={(selected) => {
                selection.set(line.id, selected);
              }}
            />
          ))}
        </Fragment>
      ))}
    </ListGrid>
  );
}
