import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { presentationOf } from "../../shared/list-view.ts";
import type { HistoryStep, SearchState } from "../../shared/search-address.ts";
import { useListView, type ResultsInput } from "../../shared/use-list-view.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { useCanonicalForms } from "./canonical-forms.ts";
import { SaveSearchButton } from "./SaveSearchButton.tsx";
import { SearchBar } from "./SearchBar.tsx";

export interface SearchPageProps {
  /** What the address says about the search. */
  readonly state: SearchState;
  /** Writes a new address; the page asks for it and the router makes it. */
  readonly onState: (state: SearchState, step: HistoryStep) => void;
  /** The list of what the search finds (WP-08): the issue list is its own area, so the page is given it. */
  readonly results: (input: ResultsInput) => ReactNode;
}

/**
 * The *Search* page (UX design section 2.1): the search bar over every project the person sees.
 * The search is in the address, so back and forward work and a shared address opens the same
 * search for anyone who may see it. The columns, the spacing and the grouping are in the address
 * too, and *Save search* keeps them with the text.
 */
export function SearchPage({ state, onState, results }: SearchPageProps) {
  usePageTitle(t("search.page.title"));
  const [stale, setStale] = useState(false);
  const forms = useCanonicalForms();
  const { view, defaults, onView } = useListView(state, onState, {
    everywhere: true,
    word: forms?.word,
  });
  const onText = (canonical: string, step: HistoryStep): void => {
    onState({ ...state, text: canonical }, step);
  };
  return (
    <Card heading={t("search.page.title")} level={1}>
      <SearchBar
        text={state.text}
        onText={onText}
        onStale={setStale}
        actions={
          <SaveSearchButton
            text={state.text}
            presentation={presentationOf(view)}
            hasMistake={stale}
          />
        }
      />
      {results({ text: state.text, onText, view, defaults, onView, stale })}
    </Card>
  );
}
