import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";
import { useState } from "react";

import { usePageTitle } from "../../shared/use-page.ts";
import { FoundIssues } from "./FoundIssues.tsx";
import { SearchBar } from "./SearchBar.tsx";
import { searchTextOf, withSearchText, type SearchAddress } from "../../shared/search-address.ts";

export interface SearchPageProps {
  /** What the address says about the search. */
  readonly address: SearchAddress;
  /** Writes a new address; the page asks for it and the router makes it. */
  readonly onAddress: (address: SearchAddress) => void;
}

/**
 * The *Search* page (UX design section 2.1): the search bar over every project the person sees.
 * The search is in the address, so back and forward work and a shared address opens the same
 * search for anyone who may see it.
 */
export function SearchPage({ address, onAddress }: SearchPageProps) {
  usePageTitle(t("search.page.title"));
  const [stale, setStale] = useState(false);
  const text = searchTextOf(address);
  return (
    <Card heading={t("search.page.title")} level={1}>
      <SearchBar
        text={text}
        onText={(canonical) => {
          onAddress(withSearchText(address, canonical));
        }}
        onStale={setStale}
      />
      <FoundIssues text={text} stale={stale} />
    </Card>
  );
}
