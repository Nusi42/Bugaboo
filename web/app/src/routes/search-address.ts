import { useLocation, useNavigate } from "@tanstack/react-router";

import {
  historyStep,
  readSearch,
  searchHref,
  type HistoryStep,
  type SearchState,
} from "../shared/search-address.ts";

/** What the address of the page that is open says about its search (the one reader, `readSearch`). */
export function useSearchState(): SearchState {
  const location = useLocation();
  return readSearch({ pathname: location.pathname, search: location.search });
}

/** Writes a search into the address (the one writer, `searchLocation`), as a new step or in place of one. */
export function useSearchNavigation(): (state: SearchState, step: HistoryStep) => void {
  const navigate = useNavigate();
  return (state, step) => {
    void navigate({ href: searchHref(state), replace: step === historyStep.replace });
  };
}
