import { useFoundGroups, useFoundIssues } from "./found-issues.ts";
import { useListContext } from "./list-context.ts";

/**
 * Whether the search of the list was refused, so that there are no results to show, not even an
 * earlier list (the address itself has a mistake). It reads the queries the list reads, which are
 * the same entries of the server-state cache.
 */
export function useListFailed(): boolean {
  const { search, words, groupBy } = useListContext();
  const rows = useFoundIssues(search, words, groupBy === null);
  const groups = useFoundGroups(search, groupBy);
  return groupBy === null ? rows.isError : groups.isError;
}
