import { Card } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

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
import { useCanonicalForms } from "./canonical-forms.ts";
import { MadeByLine } from "./MadeByLine.tsx";
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
  const forms = useCanonicalForms();
  // A search saved before D3 holds the old spelling: what it is compared with is its text as written today.
  const saved = forms === undefined ? search.search_text : forms.text(search.search_text);
  const text = textShown(state, saved);
  const { view, defaults, onView } = useListView(state, onState, {
    everywhere: true,
    presentation: search.presentation,
    word: forms?.word,
  });
  const unchanged = text === saved && !viewIsChanged(state);
  // Back at the saved text there is nothing to save, and the address is the saved search's own.
  const onText = (canonical: string, step: HistoryStep): void => {
    onState(withText(state, canonical, saved), step);
  };
  return (
    <Card heading={search.name} level={1}>
      <MadeByLine
        search={search}
        text={text}
        view={view}
        unchanged={unchanged}
        hasMistake={stale}
        onSaved={() => {
          onState(withoutChanges(state), historyStep.replace);
        }}
        onUndo={() => {
          onState(withoutChanges(state), historyStep.push);
        }}
      />
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
