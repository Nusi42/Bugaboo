import { t } from "@bugaboo/messages";

import { groupName, groupOfKey, useFoundGroups, type OwnColumns } from "../issues/index.ts";
import type { BoardRow } from "./board-data.ts";

/** What the names of the rows are made from. */
export interface RowNamesInput {
  readonly search: { readonly text: string; readonly within: string | undefined };
  /** The search word of the field the rows come from; none for a board without rows. */
  readonly rows: string | null;
  readonly own: OwnColumns;
  /** The names of the fields the person may read, by their words. */
  readonly names: Readonly<Record<string, string>>;
}

/**
 * The names of the rows of the board, the way the list names its groups: the server's name of each
 * group of the search, a person as the viewer sees them, a status group or a priority in its own words,
 * and the row without a value by what it lacks.
 */
export function useRowNames({ search, rows, own, names }: RowNamesInput) {
  const found = useFoundGroups(search, rows);
  const column = rows === null ? undefined : own[rows];
  const named = new Map(
    (found.data?.groups ?? []).map((group) => {
      const each = groupOfKey(group, column, rows ?? undefined);
      return [each.id, each] as const;
    }),
  );
  return (row: BoardRow): string => {
    if (row.key.type === "empty" && rows !== null) {
      return t("board.no-row-value", { field: names[rows] ?? rows });
    }
    const group = groupOfKey(
      { key: row.key, filter: null, name: null, issues: row.issues },
      column,
    );
    return groupName(named.get(group.id) ?? group);
  };
}
