import { loadSearchLanguage, type SearchLanguage } from "@bugaboo/wasm-search";
import { useQuery } from "@tanstack/react-query";

import { queryKeys } from "../../shared/query-keys.ts";

/**
 * The search language of the browser, fetched when a search bar first opens (ADR-0005, decision
 * 7): the module is a file of its own and no other page loads it. The cache keeps it, so every bar
 * after the first has it at once.
 */
export function useSearchLanguage(): SearchLanguage | undefined {
  return useQuery({
    queryKey: queryKeys.search.language(),
    queryFn: loadSearchLanguage,
    staleTime: Number.POSITIVE_INFINITY,
    gcTime: Number.POSITIVE_INFINITY,
  }).data;
}
