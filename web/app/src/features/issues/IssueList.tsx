import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { linesFrom } from "../../shared/paged.ts";
import { issuePanelAddress, issueAddress } from "../../shared/paths.ts";
import { IssueRow } from "./IssueRow.tsx";
import { lineOf, useProjectIssues } from "./issues.ts";
import type { IssueShow } from "./issue-show.ts";
import type { IssueSelection } from "./use-selection.ts";
import { openNewIssue } from "./new-issue-state.ts";

export interface IssueListProps {
  /** The short name of the project. */
  readonly project: string;
  readonly openIn: IssueShow;
  /** The issue that is open next to the list, by its issue ID. */
  readonly open: string | undefined;
  readonly selection: IssueSelection;
}

/** Where the title of an issue goes: the focus view, or the list with the issue in the side panel. */
function hrefOf(openIn: IssueShow, project: string, key: string): string {
  return openIn === "focus" ? issueAddress(key) : issuePanelAddress(project, key);
}

/**
 * The lean list of the issues of a project (UX §4.2, ISS-01): the newest first, a page at a time
 * with *Show more*, and a box to tick for each (WP-08 acts on the ticked ones).
 */
export function IssueList({ project, openIn, open, selection }: IssueListProps) {
  const found = useProjectIssues(project);
  const lines = linesFrom(found.data?.pages, lineOf);
  if (found.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(found.error)}</Alert>;
  }
  if (found.isSuccess && lines.length === 0) {
    return (
      <div {...partOf(part.stack)}>
        <p {...partOf(part.muted)}>{t("issues.nothing")}</p>
        <Button
          look={buttonLook.primary}
          onPress={() => {
            openNewIssue(project);
          }}
        >
          {t("issues.create-first")}
        </Button>
      </div>
    );
  }
  return (
    <div {...partOf(part.stack)}>
      <ul {...partOf(part.issueList)} aria-label={t("issues.list")}>
        {lines.map((line) => (
          <IssueRow
            key={line.id}
            line={line}
            href={hrefOf(openIn, project, line.key)}
            selected={selection.has(line.id)}
            current={line.key === open}
            onSelect={(selected) => {
              selection.set(line.id, selected);
            }}
          />
        ))}
      </ul>
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
