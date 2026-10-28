import type { SearchLanguage } from "@bugaboo/wasm-search";
import { useMemo } from "react";

import { isOwnField, phoneColumns } from "../../shared/list-columns.ts";
import type { ListView } from "../../shared/list-view.ts";
import { useFoundGroups, useFoundIssues, type FoundSearch } from "./found-issues.ts";
import { useOwnColumns, type OwnColumns } from "./own-columns.ts";

/** What the list reads: the words of its columns, the own fields they need, and why the search was refused. */
export interface ListRead {
  /** The search words of the columns, in order (a phone draws its own few). */
  readonly words: readonly string[];
  /** The own fields a column or the grouping is. */
  readonly own: OwnColumns;
  /**
   * Why the search of the list was refused, so that there are no results to show, not even an
   * earlier list (the address itself has a mistake); none while there is a list. The one place that
   * decides it: the list context holds the answer, and the head, the count line and the results
   * read it there.
   */
  readonly failure: Error | null;
}

/** Whether a column or the grouping is a field of the organization, so its fields must be read. */
function needsOwn(view: ListView): boolean {
  return view.columns.some(isOwnField) || (view.groupBy !== null && isOwnField(view.groupBy));
}

/** Reads what the list shows: the same queries as its rows and groups, which are the same entries of the server-state cache. */
export function useListRead(
  search: FoundSearch,
  view: ListView,
  language: SearchLanguage,
  phone: boolean,
): ListRead {
  const columns = view.columns.join(",");
  const words = useMemo(() => (phone ? phoneColumns : columns.split(",")), [phone, columns]);
  const own = useOwnColumns(language, needsOwn(view));
  const rows = useFoundIssues(search, words, view.groupBy === null);
  const groups = useFoundGroups(search, view.groupBy);
  const read = view.groupBy === null ? rows : groups;
  return { words, own, failure: read.isError ? read.error : null };
}
