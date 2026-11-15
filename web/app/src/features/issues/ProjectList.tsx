import { InPaneHead, paneHeadSlot } from "@bugaboo/ui";
import { useState } from "react";

import {
  textShown,
  withText,
  type HistoryStep,
  type SearchState,
} from "../../shared/search-address.ts";
import { useListView } from "../../shared/use-list-view.ts";
import { SearchBar, useCanonicalForms, useProjectListChoice } from "../search/index.ts";
import { IssueResults } from "./IssueResults.tsx";
import type { IssueShow } from "./issue-show.ts";
import { PhoneActions } from "./ProjectToolbar.tsx";
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
/**
 * The search of a project's list: its own list (the saved search a project lead chose) gives the text and the columns
 * where the address says none, and whether that is still read. A list saved before D3 holds the old spelling: it is the
 * usual text once written as today.
 */
function useProjectSearch(
  project: ListedProject,
  state: SearchState,
  onState: ProjectListProps["onState"],
) {
  const choice = useProjectListChoice(project.shortName);
  const chosen = choice.data?.search ?? undefined;
  const forms = useCanonicalForms(project.id);
  const list = useListView(state, onState, {
    everywhere: false,
    presentation: chosen?.presentation,
    word: forms?.word,
  });
  const stored = chosen?.search_text ?? "";
  const usual = forms === undefined ? stored : forms.text(stored);
  const onText = (canonical: string, step: HistoryStep): void => {
    onState(withText(state, canonical, usual), step);
  };
  return { list, text: textShown(state, usual), onText, reading: choice.isLoading };
}

export function ProjectList({ project, state, onState, openIn, selection }: ProjectListProps) {
  const [stale, setStale] = useState(false);
  const { list, text, onText, reading } = useProjectSearch(project, state, onState);
  const save = (
    <ProjectSave text={text} shortName={project.shortName} view={list.view} hasMistake={stale} />
  );
  // The bar and the list wait for the text of the project's own list, so the bar is not drawn empty and then filled
  // with its filters, pushing *Add filter* aside (UX §6.4).
  return (
    <>
      <InPaneHead slot={paneHeadSlot.search}>
        <SearchBar
          text={text}
          onText={onText}
          within={project.id}
          onStale={setStale}
          listView={{ view: list.view, onView: list.onView }}
          phoneStart={<PhoneActions shortName={project.shortName} />}
          actions={save}
          waiting={reading}
        />
      </InPaneHead>
      {reading ? null : (
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
      )}
    </>
  );
}
