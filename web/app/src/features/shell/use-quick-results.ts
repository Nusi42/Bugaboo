import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";

import { meOptions } from "../../shared/session.ts";
import { pagesSource } from "./quick-search-pages.ts";
import { useQuickSearchSources, type QuickResult } from "./quick-search-sources.ts";

/** The results of one source under its heading. */
export interface ResultGroup {
  readonly heading: string;
  readonly results: readonly QuickResult[];
}

/**
 * What the sources find for the words typed so far: the pages of the app and every source that
 * features added. A source that answers late does not replace a newer search: only the latest
 * question counts.
 */
export function useQuickResults(query: string): readonly ResultGroup[] {
  const administrator = useQuery(meOptions()).data?.administrator === true;
  const registered = useQuickSearchSources();
  const sources = useMemo(() => [pagesSource, ...registered], [registered]);
  const [groups, setGroups] = useState<readonly ResultGroup[]>([]);
  useEffect(() => {
    let latest = true;
    void Promise.all(
      sources.map(async (source): Promise<ResultGroup> => ({
        heading: source.heading,
        results: await source.search(query, { administrator }),
      })),
    ).then((found: readonly ResultGroup[]) => {
      if (latest) {
        setGroups(found.filter((group) => group.results.length > 0));
      }
    });
    return () => {
      latest = false;
    };
  }, [query, administrator, sources]);
  return groups;
}
