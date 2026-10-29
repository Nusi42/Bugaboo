import { t } from "@bugaboo/messages";
import { useMemo, type ReactNode } from "react";

import { paths, savedSearchAddress } from "../../shared/paths.ts";

import type { HistoryStep, SearchState } from "../../shared/search-address.ts";
import type { ResultsInput } from "../../shared/use-list-view.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate, usePageTrail } from "../shell/index.ts";
import { useSavedSearch, type SavedSearch } from "./saved-searches.ts";
import { SavedSearchView } from "./SavedSearchView.tsx";

export interface SavedSearchPageProps {
  /** The ID of the saved search, from the address `/s/<ID>`. */
  readonly id: string;
  /** What the address says about the search: a change that was not saved is in `?s=`. */
  readonly state: SearchState;
  readonly onState: (state: SearchState, step: HistoryStep) => void;
  /** The list of what the search finds (WP-08). */
  readonly results: (input: ResultsInput) => ReactNode;
  /** What a phone shows before *Filters* in the row of the bar (the *Legend* button of the list). */
  readonly phoneStart?: ReactNode;
  /** What is shown when the search is not there, or not for this person. */
  readonly notFound: ReactNode;
}

/**
 * The page of a saved search, `/s/<ID>` (QRY-06, ADR-0034): it opens with its text, and a change
 * is kept in the address until it is saved; leaving without saving keeps the search as it was.
 */
export function SavedSearchPage({
  id,
  state,
  onState,
  results,
  phoneStart,
  notFound,
}: SavedSearchPageProps) {
  const saved = useSavedSearch(id);
  usePageTitle(saved.data?.name ?? "");
  const name = saved.data?.name;
  const trail = useMemo(
    () =>
      name === undefined
        ? undefined
        : [
            { label: t("search.page.title"), href: paths.search },
            { label: name, href: savedSearchAddress(id) },
          ],
    [name, id],
  );
  usePageTrail(trail);
  return (
    <QueryGate error={saved.error} data={saved.data} notFound={notFound}>
      {(search: SavedSearch) => (
        <SavedSearchView
          search={search}
          state={state}
          onState={onState}
          results={results}
          phoneStart={phoneStart}
        />
      )}
    </QueryGate>
  );
}
