import { ShareButton } from "./ShareButton.tsx";
import { savedSearchActions } from "./saved-search-actions.ts";
import type { SavedSearch } from "./saved-searches.ts";

/** The buttons beside a saved search: *Share* for the person who made it, and what other areas add (a standing rule). */
export function SavedSearchActions({ search }: { readonly search: SavedSearch }) {
  return (
    <>
      {search.mine ? <ShareButton search={search} /> : null}
      {savedSearchActions().map((action) => (
        <action.component key={action.id} search={search} />
      ))}
    </>
  );
}
