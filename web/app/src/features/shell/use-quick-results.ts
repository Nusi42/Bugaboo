import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";

import { meOptions } from "../../shared/session.ts";
import { collectResults, type Collected } from "./quick-search-collect.ts";
import { pagesSource } from "./quick-search-pages.ts";
import { useQuickSearchSources } from "./quick-search-sources.ts";

const nothingYet: Collected = { groups: [], failed: false };

/**
 * What the sources find for the words typed so far: the pages of the app and every source that
 * features added. A source that answers late does not replace a newer search: only the latest
 * question counts.
 */
export function useQuickResults(query: string): Collected {
  const administrator = useQuery(meOptions()).data?.administrator === true;
  const registered = useQuickSearchSources();
  const sources = useMemo(() => [pagesSource, ...registered], [registered]);
  const [found, setFound] = useState<Collected>(nothingYet);
  useEffect(() => {
    let latest = true;
    void collectResults(sources, query, { administrator }).then((collected) => {
      if (latest) {
        setFound(collected);
      }
    });
    return () => {
      latest = false;
    };
  }, [query, administrator, sources]);
  return found;
}
