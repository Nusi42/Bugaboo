import { t } from "@bugaboo/messages";

import { listWord } from "../../shared/list-columns.ts";
import { timelineStartGroup } from "../../shared/list-view.ts";
import { groupName, groupOfKey, useFoundGroups, type OwnColumns } from "../issues/index.ts";
import type { GroupLine } from "./lines.ts";

/** What the group without a value of a grouping is called: *No goal*, *No person*, *No Severity*. */
function noValueName(groupBy: string, names: Readonly<Record<string, string>>): string {
  if (groupBy === timelineStartGroup) {
    return t("timeline.no-goal");
  }
  return groupBy === listWord.assignedTo
    ? t("timeline.no-person")
    : t("timeline.no-value", { field: names[groupBy] ?? groupBy });
}

/** What the names of the groups are made from. */
export interface GroupNamesInput {
  readonly search: { readonly text: string; readonly within: string | undefined };
  readonly groupBy: string | null;
  readonly own: OwnColumns;
  /** The names of the fields the person may read, by their words. */
  readonly names: Readonly<Record<string, string>>;
}

/**
 * The names of the groups of the timeline, the way the list names its groups: the server's name of
 * each group of the search (a goal by its Issue ID, a project, a choice …), a status group or a
 * priority in its own words, and the group without a value by what it lacks.
 */
export function useGroupNames({
  search,
  groupBy,
  own,
  names,
}: GroupNamesInput): (line: GroupLine) => string {
  const found = useFoundGroups(search, groupBy);
  const column = groupBy === null ? undefined : own[groupBy];
  const named = new Map(
    (found.data?.groups ?? []).map((group) => {
      const each = groupOfKey(group, column, groupBy ?? undefined);
      return [each.id, each] as const;
    }),
  );
  return (line) => {
    if (line.key.type === "empty" && groupBy !== null) {
      return noValueName(groupBy, names);
    }
    return groupName(
      named.get(line.id) ??
        groupOfKey({ key: line.key, filter: null, name: null, issues: line.count }, column),
    );
  };
}
