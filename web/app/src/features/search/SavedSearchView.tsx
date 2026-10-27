import { t } from "@bugaboo/messages";
import { Card, part, partOf } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { personName } from "../../shared/person.ts";
import { historyStep, type HistoryStep, type SearchState } from "../../shared/search-address.ts";
import { useListView, type ResultsInput } from "../../shared/use-list-view.ts";
import { ChangedActions } from "./ChangedActions.tsx";
import type { SavedSearch } from "./saved-searches.ts";
import { SearchBar } from "./SearchBar.tsx";
import { ShareButton } from "./ShareButton.tsx";

export interface SavedSearchViewProps {
  readonly search: SavedSearch;
  /** What the address says about the search: a change that was not saved is in `?s=`. */
  readonly state: SearchState;
  readonly onState: (state: SearchState, step: HistoryStep) => void;
  /** The list of what the search finds (WP-08): the issue list is its own area, so the page is given it. */
  readonly results: (input: ResultsInput) => ReactNode;
}

/** A saved search that was opened: its name, the bar with its text, what it finds with its columns. */
export function SavedSearchView({ search, state, onState, results }: SavedSearchViewProps) {
  const [stale, setStale] = useState(false);
  const text = state.text === "" ? search.search_text : state.text;
  const { view, defaults, onView } = useListView(state, onState, {
    everywhere: true,
    presentation: search.presentation,
  });
  // Back at the saved text there is nothing to save, and the address is the saved search's own.
  const onText = (canonical: string, step: HistoryStep): void => {
    onState({ ...state, text: canonical === search.search_text ? "" : canonical }, step);
  };
  return (
    <Card
      heading={search.name}
      level={1}
      actions={
        text === search.search_text ? null : (
          <ChangedActions
            search={search}
            text={text}
            hasMistake={stale}
            onSaved={() => {
              onState({ ...state, text: "" }, historyStep.replace);
            }}
          />
        )
      }
    >
      <p {...partOf(part.muted)}>{t("search.made-by", { person: personName(search.maker) })}</p>
      <SearchBar
        text={text}
        onText={onText}
        onStale={setStale}
        actions={search.mine ? <ShareButton search={search} /> : null}
      />
      {results({ text, onText, view, defaults, onView, stale })}
    </Card>
  );
}
