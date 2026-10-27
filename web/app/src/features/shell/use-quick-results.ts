import { useEffect, useMemo, useRef, useState } from "react";

import { useLater } from "../../shared/use-later.ts";
import { usePermissions } from "../../shared/use-me.ts";
import { useSignOut } from "../../shared/use-sign-out.ts";
import { actionsSource } from "./quick-search-actions.ts";
import { arrange, collectResults, markStale, type Collected } from "./quick-search-collect.ts";
import { pagesSource } from "./quick-search-pages.ts";
import { savedSearchesSource } from "./quick-search-saved.ts";
import {
  useQuickSearchSources,
  type QuickSearchAsker,
  type QuickSearchSource,
} from "./quick-search-sources.ts";

const nothingYet: Collected = { groups: [], failed: false };

/** How long the person has to stop typing before the server is asked (M5-Q10). */
export const typingPauseMs = 250;

/** The text once the person has stopped typing for [`typingPauseMs`]; at first it is the text itself. */
function useSettled(text: string): string {
  const later = useLater();
  const [settled, setSettled] = useState(text);
  useEffect(() => {
    later.schedule(typingPauseMs, () => {
      setSettled(text);
    });
  }, [text, later]);
  return settled;
}

/** What sources found, and the words they were asked for. */
interface Answered {
  readonly found: Collected;
  /** The words of the question the answer is for; nothing before the first answer. */
  readonly to: string | null;
}

/**
 * What `sources` find for `query`. A source that answers late does not replace a newer search:
 * only the latest question counts.
 */
function useCollected(
  sources: readonly QuickSearchSource[],
  query: string,
  asker: QuickSearchAsker,
): Answered {
  const [answered, setAnswered] = useState<Answered>({ found: nothingYet, to: null });
  useEffect(() => {
    let latest = true;
    void collectResults(sources, query, asker).then((collected) => {
      if (latest) {
        setAnswered({ found: collected, to: query });
      }
    });
    return () => {
      latest = false;
    };
  }, [query, asker, sources]);
  return answered;
}

/** What quick search found, and whether the sources that ask the server have yet to answer for these words. */
export interface QuickFound extends Collected {
  readonly waiting: boolean;
  /** The person has stopped typing for [`typingPauseMs`]: the words are the ones the server is asked for. */
  readonly quiet: boolean;
}

/**
 * What the sources find for the words typed so far: the pages and actions of the app and every
 * source that features added. The sources that answer at once follow every key; the ones that ask
 * the server wait for a pause in typing, so a search is counted once for each pause (M5-Q10).
 */
export function useQuickResults(query: string): QuickFound {
  const permissions = usePermissions();
  const { signOut } = useSignOut();
  // The way to sign out is new with every render; the question to the sources must not be, or
  // every answer would ask again.
  const latestSignOut = useRef(signOut);
  useEffect(() => {
    latestSignOut.current = signOut;
  }, [signOut]);
  const asker = useMemo(
    () => ({
      permissions,
      signOut: () => {
        latestSignOut.current();
      },
    }),
    [permissions],
  );
  const registered = useQuickSearchSources();
  const sources = useMemo(
    () => [pagesSource, actionsSource, savedSearchesSource, ...registered],
    [registered],
  );
  const nearby = useMemo(() => sources.filter((source) => source.paused !== true), [sources]);
  const far = useMemo(() => sources.filter((source) => source.paused === true), [sources]);
  const settled = useSettled(query);
  const now = useCollected(nearby, query, asker);
  const later = useCollected(far, settled, asker);
  return useMemo(() => {
    // What the server found for older words keeps its place while it is asked again, so no group
    // goes and comes back, but it is marked busy and its rows cannot be chosen.
    const waiting = later.to !== query;
    return {
      ...arrange(sources, query, now.found, markStale(later.found, waiting)),
      waiting,
      quiet: settled === query,
    };
  }, [sources, query, settled, now, later]);
}
