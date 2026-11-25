import { PinButton, pinKind } from "../pins/index.ts";
import { SearchMailButton } from "./SearchMailButton.tsx";
import { ShareButton } from "./ShareButton.tsx";
import { savedSearchActions } from "./saved-search-actions.ts";
import type { SavedSearch } from "./saved-searches.ts";

/**
 * The buttons beside a saved search: *Pin to sidebar*, *Share* for the person who made it, *E-mail me this search* for
 * everyone who may open it, and what other areas add (a standing rule).
 */
export function SavedSearchActions({ search }: { readonly search: SavedSearch }) {
  return (
    <>
      <PinButton kind={pinKind.savedSearch} target={search.id} />
      {search.mine ? <ShareButton search={search} /> : null}
      <SearchMailButton search={search} />
      {savedSearchActions().map((action) => (
        <action.component key={action.id} search={search} />
      ))}
    </>
  );
}
