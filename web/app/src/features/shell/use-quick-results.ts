import { useEffect, useMemo, useState } from "react";

import { usePermissions } from "../../shared/use-me.ts";
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
  const permissions = usePermissions();
  const registered = useQuickSearchSources();
  const sources = useMemo(() => [pagesSource, ...registered], [registered]);
  const [found, setFound] = useState<Collected>(nothingYet);
  useEffect(() => {
    let latest = true;
    void collectResults(sources, query, { permissions }).then((collected) => {
      if (latest) {
        setFound(collected);
      }
    });
    return () => {
      latest = false;
    };
  }, [query, permissions, sources]);
  return found;
}
