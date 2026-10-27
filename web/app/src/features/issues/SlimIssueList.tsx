import { t } from "@bugaboo/messages";
import { listRowLook, listShowsWord, ListGrid, ListRow, Priority } from "@bugaboo/ui";

import { issueAddress } from "../../shared/paths.ts";
import { issueCells } from "./issue-cells.tsx";
import type { IssueLine } from "./issues.ts";
import { slimColumns } from "./slim-columns.ts";

export interface SlimIssueListProps {
  readonly lines: readonly IssueLine[];
  /** The issue that is open, by its issue ID. */
  readonly open: string;
}

/**
 * The slim list on the left of the focus view (UX §3.4): the rows of the issue list in two lines,
 * the status icon, the issue ID and the priority above the title, left-aligned, with the tint and
 * the edge of the row. The issue that is open is marked, and a whole row opens its issue.
 */
export function SlimIssueList({ lines, open }: SlimIssueListProps) {
  return (
    <ListGrid label={t("issues.list")}>
      {lines.map((line) => (
        <ListRow
          key={line.id}
          variant={listRowLook.slim}
          columns={slimColumns}
          group={line.group}
          priority={line.priority}
          current={line.key === open}
          cells={{
            ...issueCells(
              {
                key: line.key,
                title: line.title,
                group: line.group,
                statusWord: line.status.name,
                type: line.issueType,
              },
              issueAddress(line.key),
              { current: line.key === open },
            ),
            priority: (
              <Priority
                priority={line.priority}
                label={t(`priority.${line.priority}`)}
                showWord={listShowsWord(line.priority)}
              />
            ),
          }}
        />
      ))}
    </ListGrid>
  );
}
