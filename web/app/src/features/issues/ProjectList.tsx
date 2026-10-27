import { useState } from "react";

import type { HistoryStep, SearchState } from "../../shared/search-address.ts";
import { useListView } from "../../shared/use-list-view.ts";
import { SearchBar, useProjectListChoice } from "../search/index.ts";
import { IssueResults } from "./IssueResults.tsx";
import type { IssueShow } from "./issue-show.ts";
import { ProjectSave } from "./ProjectSave.tsx";
import type { IssueSelection } from "./use-selection.ts";

/** The project a list shows: its ID for the search, its short name for the addresses. */
export interface ListedProject {
  readonly id: string;
  readonly shortName: string;
  readonly name: string;
}

export interface ProjectListProps {
  readonly project: ListedProject;
  /** What the address says about the search. */
  readonly state: SearchState;
  readonly onState: (state: SearchState, step: HistoryStep) => void;
  readonly openIn: IssueShow;
  readonly selection: IssueSelection;
}

/**
 * The issues of a project (UX section 4.2): the search bar with the search `project within` filled
 * in, and under it the list. The project's own list, the saved search that a project lead chose
 * (ADR-0034 decision 6), gives the text and the columns where the address says none.
 */
export function ProjectList({ project, state, onState, openIn, selection }: ProjectListProps) {
  const [stale, setStale] = useState(false);
  const chosen = useProjectListChoice(project.shortName).data?.search ?? undefined;
  const list = useListView(state, onState, {
    everywhere: false,
    presentation: chosen?.presentation,
  });
  const usual = chosen?.search_text ?? "";
  const text = state.text === "" ? usual : state.text;
  const onText = (canonical: string, step: HistoryStep): void => {
    onState({ ...state, text: canonical === usual ? "" : canonical }, step);
  };
  const save = (
    <ProjectSave text={text} shortName={project.shortName} view={list.view} hasMistake={stale} />
  );
  return (
    <>
      <SearchBar
        text={text}
        onText={onText}
        within={project.id}
        onStale={setStale}
        actions={save}
      />
      <IssueResults
        {...list}
        text={text}
        onText={onText}
        stale={stale}
        within={project.id}
        scopeName={project.name}
        project={project.shortName}
        openIn={openIn}
        state={state}
        open={state.open}
        selection={selection}
      />
    </>
  );
}
