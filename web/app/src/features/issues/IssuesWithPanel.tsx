import { t } from "@bugaboo/messages";
import { Card, cardLook, partOf, part } from "@bugaboo/ui";

import type { HistoryStep, SearchState } from "../../shared/search-address.ts";

import { OpenIssue } from "./OpenIssue.tsx";
import { useOpenIssuesIn } from "./open-in.ts";
import { ProjectList, type ListedProject } from "./ProjectList.tsx";
import { ProjectToolbar } from "./ProjectToolbar.tsx";
import { useIssueOnPhone } from "./use-issue-on-phone.ts";
import { useSelection } from "./use-selection.ts";

export interface IssuesWithPanelProps {
  readonly project: ListedProject;
  /** What the address says about the search and how its results are shown. */
  readonly state: SearchState;
  readonly onState: (state: SearchState, step: HistoryStep) => void;
  /** Opens an issue in the focus view. */
  readonly onFocus: (key: string) => void;
  /** Closes the side panel. */
  readonly onClose: () => void;
}

/** The list of a project and, beside it, the issue that is open in the side panel. */
export function IssuesWithPanel({
  project,
  state,
  onState,
  onFocus,
  onClose,
}: IssuesWithPanelProps) {
  const openIn = useOpenIssuesIn();
  const { open } = state;
  const selection = useSelection();
  const toPage = useIssueOnPhone(state);
  if (toPage) {
    return null;
  }
  return (
    <Card
      heading={t("issues.page-title", { project: project.name })}
      level={1}
      look={cardLook.openOnPhone}
    >
      <div {...partOf(part.listEditor)}>
        <div {...partOf(part.listEditorList)}>
          <ProjectToolbar shortName={project.shortName} beside={open !== undefined} />
          <ProjectList
            project={project}
            state={state}
            onState={onState}
            openIn={openIn}
            selection={selection}
          />
        </div>
        {open === undefined ? null : <OpenIssue open={open} onFocus={onFocus} onClose={onClose} />}
      </div>
    </Card>
  );
}
