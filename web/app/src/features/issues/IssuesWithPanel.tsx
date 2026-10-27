import { t } from "@bugaboo/messages";
import { Card, cardLook, useLayout, partOf, part } from "@bugaboo/ui";

import type { HistoryStep, SearchState } from "../../shared/search-address.ts";

import { OpenIssue } from "./OpenIssue.tsx";
import { useOpenIssuesIn } from "./open-in.ts";
import { ProjectList, type ListedProject } from "./ProjectList.tsx";
import { ProjectToolbar } from "./ProjectToolbar.tsx";
import { useSelection } from "./use-selection.ts";

export interface IssuesWithPanelProps {
  readonly project: ListedProject;
  /** What the address says about the search and how its results are shown. */
  readonly state: SearchState;
  readonly onState: (state: SearchState, step: HistoryStep) => void;
  /** The issue open in the side panel, from the address. */
  readonly open: string | undefined;
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
  open,
  onFocus,
  onClose,
}: IssuesWithPanelProps) {
  const openIn = useOpenIssuesIn();
  const selection = useSelection();
  // A phone shows one thing at a time: the open issue takes the screen and Back closes it.
  const layout = useLayout();
  const alone = layout === "phone" && open !== undefined;
  return (
    <Card
      heading={t("issues.page-title", { project: project.name })}
      level={1}
      look={cardLook.openOnPhone}
    >
      <div {...partOf(part.listEditor)}>
        <div {...partOf(part.listEditorList, { states: [alone && "hidden"] })} hidden={alone}>
          <ProjectToolbar shortName={project.shortName} selection={selection} />
          <ProjectList
            project={project}
            state={state}
            onState={onState}
            openIn={openIn}
            open={open}
            selection={selection}
          />
        </div>
        {open === undefined ? null : <OpenIssue open={open} onFocus={onFocus} onClose={onClose} />}
      </div>
    </Card>
  );
}
