import { groupName, groupOfKey, useFoundGroups, type OwnColumns } from "../issues/index.ts";
import type { GroupLine } from "./lines.ts";

/**
 * The names of the groups of the timeline, the way the list names its groups: the server's name of
 * each group of the search (a goal by its Issue ID, a project, a choice …), a status group or a
 * priority in its own words.
 */
export function useGroupNames(
  search: { readonly text: string; readonly within: string | undefined },
  groupBy: string | null,
  own: OwnColumns,
): (line: GroupLine) => string {
  const found = useFoundGroups(search, groupBy);
  const column = groupBy === null ? undefined : own[groupBy];
  const named = new Map(
    (found.data?.groups ?? []).map((group) => {
      const each = groupOfKey(group, column, groupBy ?? undefined);
      return [each.id, each] as const;
    }),
  );
  return (line) =>
    groupName(
      named.get(line.id) ??
        groupOfKey({ key: line.key, filter: null, name: null, issues: line.count }, column),
    );
}
