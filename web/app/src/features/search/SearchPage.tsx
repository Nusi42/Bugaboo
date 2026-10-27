import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";
import { useState } from "react";

import type { HistoryStep, SearchState } from "../../shared/search-address.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { FoundIssues } from "./FoundIssues.tsx";
import { SaveSearchButton } from "./SaveSearchButton.tsx";
import { SearchBar } from "./SearchBar.tsx";

export interface SearchPageProps {
  /** What the address says about the search. */
  readonly state: SearchState;
  /** Writes a new address; the page asks for it and the router makes it. */
  readonly onState: (state: SearchState, step: HistoryStep) => void;
}

/**
 * The *Search* page (UX design section 2.1): the search bar over every project the person sees.
 * The search is in the address, so back and forward work and a shared address opens the same
 * search for anyone who may see it.
 */
export function SearchPage({ state, onState }: SearchPageProps) {
  usePageTitle(t("search.page.title"));
  const [stale, setStale] = useState(false);
  return (
    <Card heading={t("search.page.title")} level={1}>
      <SearchBar
        text={state.text}
        onText={(canonical, step) => {
          onState({ ...state, text: canonical }, step);
        }}
        onStale={setStale}
        actions={<SaveSearchButton text={state.text} hasMistake={stale} />}
      />
      <FoundIssues text={state.text} stale={stale} />
    </Card>
  );
}
