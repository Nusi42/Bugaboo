import { t } from "@bugaboo/messages";
import { Card, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { personName } from "../../shared/person.ts";
import { withSearchText, type SearchAddress } from "../../shared/search-address.ts";
import { ChangedActions } from "./ChangedActions.tsx";
import { FoundIssues } from "./FoundIssues.tsx";
import type { SavedSearch } from "./saved-searches.ts";
import { SearchBar } from "./SearchBar.tsx";
import { ShareButton } from "./ShareButton.tsx";

export interface SavedSearchViewProps {
  readonly search: SavedSearch;
  /** What the address says about the search: a change that was not saved is in `?s=`. */
  readonly address: SearchAddress;
  readonly onAddress: (address: SearchAddress) => void;
}

/** A saved search that was opened: its name, the bar with its text, what it finds. */
export function SavedSearchView({ search, address, onAddress }: SavedSearchViewProps) {
  const [stale, setStale] = useState(false);
  const text = address.s ?? search.search_text;
  return (
    <Card
      heading={search.name}
      level={1}
      actions={text === search.search_text ? null : <ChangedActions search={search} text={text} />}
    >
      <p {...partOf(part.muted)}>{t("search.made-by", { person: personName(search.maker) })}</p>
      <SearchBar
        text={text}
        onText={(canonical) => {
          // Back at the saved text there is nothing to save, and the address is the saved search's own.
          onAddress(withSearchText(address, canonical === search.search_text ? "" : canonical));
        }}
        onStale={setStale}
        actions={search.mine ? <ShareButton search={search} /> : null}
      />
      <FoundIssues text={text} stale={stale} />
    </Card>
  );
}
