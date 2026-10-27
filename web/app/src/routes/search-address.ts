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

/** The part of the address that is not the search: the issue open in the side panel of a project's list. */
const openParam = "open";

/**
 * Writes a search into the address (the one writer, `searchLocation`), as a new step or in place of
 * one. The issue that is open in the side panel stays open when the search changes.
 */
export function useSearchNavigation(
  open?: string,
): (state: SearchState, step: HistoryStep) => void {
  const navigate = useNavigate();
  return (state, step) => {
    const href = searchHref(state);
    const kept =
      open === undefined
        ? ""
        : `${href.includes("?") ? "&" : "?"}${openParam}=${encodeURIComponent(open)}`;
    void navigate({ href: `${href}${kept}`, replace: step === historyStep.replace });
  };
}
