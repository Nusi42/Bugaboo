import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { linesFrom } from "../../shared/paged.ts";
import { withFilter } from "../../shared/with-filter.ts";
import { foundLineOf, useFoundIssues, type FoundLine } from "./found-issues.ts";
import type { GroupOfKey } from "./group-keys.ts";
import { useListContext } from "./list-context.ts";

/** The rows of one group of a list. */
export interface GroupRows {
  /** Whether the rows can be asked for: a group the language cannot name has no filter. */
  readonly listable: boolean;
  readonly lines: readonly FoundLine[];
  /** Why the rows could not be read, in words. */
  readonly failed: string | undefined;
  /** There may be more to ask for: the first page has not been asked for, or there is a next one. */
  readonly hasMore: boolean;
  readonly isLoading: boolean;
  /** Asks for the first page, or the next. */
  readonly onMore: () => void;
}

/**
 * The rows of a group: the same search with the filter of the group added. They are asked for when
 * the group is open and its place has come into view, so a folded group and a group far down the
 * list load nothing.
 */
export function useGroupRows(group: GroupOfKey, open: boolean): GroupRows {
  const { search, groupBy, language, words } = useListContext();
  const [asked, setAsked] = useState(false);
  const text =
    group.filter === undefined || groupBy === null
      ? undefined
      : withFilter(language, search.text, groupBy, group.filter);
  const found = useFoundIssues(
    { text: text ?? "", within: search.within },
    words,
    open && asked && text !== undefined,
  );
  return {
    listable: text !== undefined,
    lines: linesFrom(found.data?.pages, foundLineOf),
    failed: found.isError ? messageOf(found.error) : undefined,
    hasMore: !asked || found.hasNextPage,
    isLoading: found.isFetching,
    onMore: () => {
      if (!asked) {
        setAsked(true);
      } else if (!found.isFetchingNextPage) {
        void found.fetchNextPage();
      }
    },
  };
}
