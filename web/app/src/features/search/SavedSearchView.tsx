import { t } from "@bugaboo/messages";
import { Card, part, partOf } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { personName } from "../../shared/person.ts";
import {
  historyStep,
  textShown,
  withText,
  type HistoryStep,
  type SearchState,
} from "../../shared/search-address.ts";
import {
  useListView,
  viewIsChanged,
  withoutChanges,
  type ResultsInput,
} from "../../shared/use-list-view.ts";
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
  const text = textShown(state, search.search_text);
  const { view, defaults, onView } = useListView(state, onState, {
    everywhere: true,
    presentation: search.presentation,
  });
  const unchanged = text === search.search_text && !viewIsChanged(state);
  // Back at the saved text there is nothing to save, and the address is the saved search's own.
  const onText = (canonical: string, step: HistoryStep): void => {
    onState(withText(state, canonical, search.search_text), step);
  };
  return (
    <Card
      heading={search.name}
      level={1}
      actions={
        <span
          {...partOf(part.searchHeldActions, { states: [unchanged && "held"] })}
          inert={unchanged}
        >
          <ChangedActions
            search={search}
            text={text}
            view={view}
            hasMistake={stale}
            onSaved={() => {
              onState(withoutChanges(state), historyStep.replace);
            }}
            onUndo={() => {
              onState(withoutChanges(state), historyStep.push);
            }}
          />
        </span>
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
