import { projectPages } from "../../shared/project-pages.ts";
import type { HistoryStep, SearchState } from "../../shared/search-address.ts";
import { ProjectGate } from "../projects/index.ts";
import { IssuesWithPanel } from "./IssuesWithPanel.tsx";

export interface ProjectIssuesPageProps {
  /** The project as the address names it: a short name (also an old one) or an ID. */
  readonly address: string;
  /** What the address says about the search and how its results are shown. */
  readonly state: SearchState;
  readonly onState: (state: SearchState, step: HistoryStep) => void;
  /** Opens an issue in the focus view. */
  readonly onFocus: (key: string) => void;
  /** Closes the side panel. */
  readonly onClose: () => void;
}

/**
 * The issues of a project (ISS-01, UX §4.2): the lean list with a box to tick for each issue, the
 * issue that is open in the side panel when the person chose that view, and *New issue*. A
 * project the person may not see is *Not found*, like one that does not exist.
 */
export function ProjectIssuesPage({
  address,
  state,
  onState,
  onFocus,
  onClose,
}: ProjectIssuesPageProps) {
  return (
    <ProjectGate address={address} page={projectPages.issues}>
      {(project) => (
        <IssuesWithPanel
          project={{
            id: project.project.id,
            shortName: project.project.short_name,
            name: project.project.name,
          }}
          state={state}
          onState={onState}
          onFocus={onFocus}
          onClose={onClose}
        />
      )}
    </ProjectGate>
  );
}
