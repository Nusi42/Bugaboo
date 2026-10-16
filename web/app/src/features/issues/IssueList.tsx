import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { linesFrom } from "../../shared/paged.ts";
import type { IssueShow } from "./issue-show.ts";
import { IssueGrid } from "./IssueGrid.tsx";
import { lineOf, useProjectIssues } from "./issues.ts";
import { NoIssues } from "./NoIssues.tsx";
import { useRowKeys } from "./use-row-keys.ts";
import type { IssueSelection } from "./use-selection.ts";

export interface IssueListProps {
  /** The short name of the project. */
  readonly project: string;
  readonly openIn: IssueShow;
  /** The issue that is open next to the list, by its issue ID. */
  readonly open: string | undefined;
  readonly selection: IssueSelection;
}

/**
 * The lean list of the issues of a project (UX §4.2, ISS-01): a grid in groups by status group, a
 * page at a time with *Show more*, and a box to tick for each (WP-08 acts on the ticked ones).
 */
export function IssueList({ project, openIn, open, selection }: IssueListProps) {
  const found = useProjectIssues(project);
  const rows = useRowKeys();
  const lines = linesFrom(found.data?.pages, lineOf);
  if (found.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(found.error)}</Alert>;
  }
  if (found.isSuccess && lines.length === 0) {
    return <NoIssues project={project} />;
  }
  return (
    <div ref={rows} {...partOf(part.stack)}>
      <IssueGrid
        lines={lines}
        project={project}
        openIn={openIn}
        open={open}
        selection={selection}
      />
      {found.hasNextPage ? (
        <Button
          isPending={found.isFetchingNextPage}
          onPress={() => {
            void found.fetchNextPage();
          }}
        >
          {t("issues.more-issues")}
        </Button>
      ) : null}
    </div>
  );
}
