import { historyTabWords, type Schema } from "@bugaboo/api-client";
import { useEffect, useRef, type RefObject } from "react";
import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { whoOf } from "../../shared/change-sentence.ts";
import { useIssueTarget } from "../../shared/issue-target.ts";
import type { IssueTarget } from "../../shared/paths.ts";
import { queryKeys } from "../../shared/query-keys.ts";

export { changeSentence, sideOf, whoOf } from "../../shared/change-sentence.ts";

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
 * The index of the entry *Open* of a notification came to, among the entries of the tab, or -1.
 * While it is not among them and there are earlier pages, the next page is read: the entry may be
 * further back than the first page goes.
 */
export function useOpenedEntry(
  groups: readonly (readonly HistoryEntry[])[],
  list: Readonly<{
    hasNextPage: boolean;
    isFetchingNextPage: boolean;
    fetchNextPage: () => Promise<unknown>;
  }>,
): number {
  const target = useIssueTarget();
  const found = groups.findIndex((group) => isTarget(group, target));
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = list;
  useEffect(() => {
    if (target !== undefined && found < 0 && hasNextPage && !isFetchingNextPage) {
      void fetchNextPage();
    }
  }, [target, found, hasNextPage, isFetchingNextPage, fetchNextPage]);
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

/** The sentence of a relation that was added or taken away. */
export function relationSentence(entry: HistoryEntry): string {
  const message =
    entry.relation === "added" ? "history.relation-added" : "history.relation-removed";
  return t(message, {
    who: whoOf(entry),
    name: entry.field ?? "",
    issue: entry.issue?.key ?? "",
  });
}
