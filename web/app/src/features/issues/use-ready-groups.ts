import type { ReadonlyDeep } from "@bugaboo/api-client";
import { useEffect, useState } from "react";

import { doNothing } from "../../shared/do-nothing.ts";
import { withGroupFilter } from "../../shared/with-filter.ts";
import { readFirstFound } from "./found-reads.ts";
import { groupsOfKeys, type GroupOfKey } from "./group-keys.ts";
import { useListContext } from "./list-context.ts";
import type { useFoundGroups } from "./found-issues.ts";

/** How many groups from the top come with their first rows: those that fill the first screen. */
const LEADING = 3;

/** The groups of a list as they are drawn, and what the answer says about all of them. */
export interface ReadyGroups {
  readonly groups: readonly GroupOfKey[] | undefined;
  readonly empty: boolean;
  readonly truncated: boolean;
}

/** What is drawn: the groups of one answer, with the grouping they were made for. */
interface Drawn {
  readonly data: ReadonlyDeep<NonNullable<ReturnType<typeof useFoundGroups>["data"]>>;
  readonly groupBy: string | null;
}

/**
 * The groups of the list to draw (UX §6.4: a list keeps what it shows until the new answer is
 * there). New groups (another grouping, another search) are drawn once the first rows of the
 * groups at the top are read too, so the heads and their rows come in one frame; until then the
 * groups before stay. The groups further down read their rows when they come into view. `answer`
 * is the newest answer of the groups, not one kept from a search before.
 */
export function useReadyGroups(answer: Drawn["data"] | undefined): ReadyGroups {
  const { search, groupBy, own, language, words } = useListContext();
  const [drawn, setDrawn] = useState<Drawn | null>(null);
  useEffect(() => {
    if (answer === undefined || answer === drawn?.data) {
      return doNothing;
    }
    let current = true;
    const leading = groupsOfKeys(
      answer.groups,
      groupBy === null ? undefined : own[groupBy],
      groupBy ?? undefined,
    )
      .slice(0, LEADING)
      .flatMap((group) => {
        const text =
          group.filter === undefined
            ? undefined
            : withGroupFilter(language, search.text, group.filter);
        return text === undefined ? [] : [readFirstFound({ text, within: search.within }, words)];
      });
    void Promise.all(leading).then(() => {
      if (current) {
        setDrawn({ data: answer, groupBy });
      }
    });
    return () => {
      current = false;
    };
  }, [answer, drawn, groupBy, own, language, search, words]);
  return {
    groups:
      drawn === null
        ? undefined
        : groupsOfKeys(
            drawn.data.groups,
            drawn.groupBy === null ? undefined : own[drawn.groupBy],
            drawn.groupBy ?? undefined,
          ),
    empty: drawn?.data.groups.length === 0,
    truncated: drawn?.data.truncated === true,
  };
}
