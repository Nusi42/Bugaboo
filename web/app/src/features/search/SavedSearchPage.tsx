import type { ReactNode } from "react";

import type { SearchAddress } from "../../shared/search-address.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
import { useSavedSearch, type SavedSearch } from "./saved-searches.ts";
import { SavedSearchView } from "./SavedSearchView.tsx";

export interface SavedSearchPageProps {
  /** The ID of the saved search, from the address `/s/<ID>`. */
  readonly id: string;
  /** What the address says about the search: a change that was not saved is in `?s=`. */
  readonly address: SearchAddress;
  readonly onAddress: (address: SearchAddress) => void;
  /** What is shown when the search is not there, or not for this person. */
  readonly notFound: ReactNode;
}

/**
 * The page of a saved search, `/s/<ID>` (QRY-06, ADR-0034): it opens with its text, and a change
 * is kept in the address until it is saved; leaving without saving keeps the search as it was.
 */
export function SavedSearchPage({ id, address, onAddress, notFound }: SavedSearchPageProps) {
  const saved = useSavedSearch(id);
  usePageTitle(saved.data?.name ?? "");
  return (
    <QueryGate error={saved.error} data={saved.data} notFound={notFound}>
      {(search: SavedSearch) => (
        <SavedSearchView search={search} address={address} onAddress={onAddress} />
      )}
    </QueryGate>
  );
}
