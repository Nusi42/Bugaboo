import type { PagedList } from "@bugaboo/ui";
import { historyTabWords, type Schema } from "@bugaboo/api-client";
import { useEffect, useRef, type RefObject } from "react";
import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { whoOf } from "../../shared/change-sentence.ts";
import { useIssueTarget } from "../../shared/issue-target.ts";
import type { IssueTarget } from "../../shared/paths.ts";
import { queryKeys } from "../../shared/query-keys.ts";

export { changeSentence, whoOf } from "../../shared/change-sentence.ts";

/** One item of the history of an issue: a comment, one field that changed, or a relation. */
export type HistoryEntry = Schema<"HistoryEntryItem">;

/** The tabs of the history, in the order they are shown (the words the server lists). */
export const historyTabs = historyTabWords;
export type HistoryTabName = Schema<"HistoryTab">;

/** The history of an issue, newest first, a page at a time. */
export function useHistory(issue: string, tab: HistoryTabName) {
  return usePaged({
    queryKey: queryKeys.history.list(issue, tab),
    read: (after) =>
      api.client.GET("/api/v1/issues/{issue}/history", {
        params: { path: { issue }, query: { tab, limit: pageSize.short, ...after } },
      }),
  });
}

/** The items of one event, which the server lists one after the other, as one entry. */
export function groupByEvent(items: readonly HistoryEntry[]): readonly (readonly HistoryEntry[])[] {
  const groups: HistoryEntry[][] = [];
  for (const item of items) {
    const last = groups.at(-1);
    if (last !== undefined && last[0]?.event === item.event && item.kind === "change") {
      last.push(item);
    } else {
      groups.push([item]);
    }
  }
  return groups;
}

/**
 * Whether an entry of the history (the items of one event) is where *Open* of a notification goes:
 * the comment with that ID, or the change that happened at that moment (compared to the
 * millisecond, however many digits the moment is written with).
 */
export function isTarget(entry: readonly HistoryEntry[], target?: IssueTarget): boolean {
  const first = entry[0];
  if (first === undefined || target === undefined) {
    return false;
  }
  if (target.kind === "comment") {
    return first.kind === "comment" && first.comment === target.comment;
  }
  return first.kind !== "comment" && Date.parse(first.at) === Date.parse(target.at);
}

/**
 * How many pages *Open* of a notification reads at most while it looks for its entry. An entry
 * further back than this is not brought to the screen: the person finds it with *Show earlier*.
 */
export const openedPagesMax = 10;

/** Whether the tab can hold the entry: a comment is in *Everything* and *Comments*, a change in *Everything* and *Changes*. */
function tabCanHold(tab: HistoryTabName, target: IssueTarget): boolean {
  return tab === "everything" || tab === (target.kind === "comment" ? "comments" : "changes");
}

/** What decides whether *Open* of a notification reads one more page. */
export interface OpenedSearch extends Pick<PagedList, "hasNextPage" | "isFetchingNextPage"> {
  readonly target?: IssueTarget | undefined;
  readonly tab: HistoryTabName;
  readonly found: boolean;
  readonly pagesRead: number;
}

/**
 * Whether the next page is read for *Open* of a notification: only in a tab that can hold the
 * entry, only for a target that can be told from others (a moment that does not parse matches
 * nothing), only while it is not found and there are earlier pages, and not past
 * {@link openedPagesMax} pages.
 */
export function readsEarlierForTarget(search: OpenedSearch): boolean {
  const { target } = search;
  if (target === undefined || search.found || !search.hasNextPage || search.isFetchingNextPage) {
    return false;
  }
  if (target.kind === "change" && Number.isNaN(Date.parse(target.at))) {
    return false;
  }
  return tabCanHold(search.tab, target) && search.pagesRead < openedPagesMax;
}

/**
 * The index of the entry *Open* of a notification came to, among the entries of the tab, or -1.
 * While it is not among them and there are earlier pages (up to {@link openedPagesMax}), the next
 * page is read: the entry may be further back than the first page goes.
 */
export function useOpenedEntry(
  groups: readonly (readonly HistoryEntry[])[],
  tab: HistoryTabName,
  list: Readonly<PagedList & { pagesRead: number }>,
): number {
  const target = useIssueTarget();
  const found = groups.findIndex((group) => isTarget(group, target));
  const { pagesRead, hasNextPage, isFetchingNextPage, fetchNextPage } = list;
  const more = readsEarlierForTarget({
    target,
    tab,
    found: found >= 0,
    pagesRead,
    hasNextPage,
    isFetchingNextPage,
  });
  useEffect(() => {
    if (more) {
      void fetchNextPage();
    }
  }, [more, fetchNextPage]);
  return found;
}

/**
 * Brings the element into view once, when `wanted` becomes true: the entry *Open* of a
 * notification came to. Gives back the ref to put on the element.
 */
export function useScrollTo<Element extends HTMLElement>(
  wanted: boolean,
): RefObject<Element | null> {
  const element = useRef<Element>(null);
  useEffect(() => {
    if (wanted) {
      element.current?.scrollIntoView({ block: "center" });
    }
  }, [wanted]);
  return element;
}

/**
 * The sentence of an item that is a relation; nothing for a change of a field or a file, whose
 * words come from the shared wording (`changeSentence`, `fileSentence`), which has to be loaded
 * first.
 */
export function plainSentenceOf(entry: HistoryEntry): string | undefined {
  return entry.kind === "relation" ? relationSentence(entry) : undefined;
}

/** The sentence of a relation that was added or taken away. */
export function relationSentence(entry: HistoryEntry): string {
  const message =
    entry.relation_event === "added" ? "history.relation-added" : "history.relation-removed";
  return t(message, {
    who: whoOf(entry),
    name: entry.relation_name ?? "",
    issue: entry.issue?.readable_id ?? "",
  });
}
