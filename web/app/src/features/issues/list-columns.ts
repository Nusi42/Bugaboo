import { listColumns, type ListColumn } from "@bugaboo/ui";

/** The columns of the list that are not a choice: the tick box and the buttons of the row. */
const fixedColumns: ReadonlySet<ListColumn> = new Set<ListColumn>(["select", "action"]);

/** The columns that cannot be taken away in *Customize → Columns* (UX §4.2). */
const keptColumns: readonly ListColumn[] = ["key", "title"];

/** The columns a person can choose from, in the order the project's list has them (ADR-0034 decision 6). */
export const projectColumns: readonly ListColumn[] = listColumns.filter(
  (column) => !fixedColumns.has(column),
);

/**
 * The columns after *Customize → Columns*: the ticked ones, in the order given, and the ID and the
 * title whatever was ticked (they come first when they were not ticked). A column that is not a
 * choice is left out.
 */
export function chooseColumns(ticked: readonly ListColumn[]): readonly ListColumn[] {
  const chosen = ticked.filter((column) => projectColumns.includes(column));
  const missing = keptColumns.filter((column) => !chosen.includes(column));
  return [...missing, ...chosen];
}

/** One column one place up or down; at an end it stays. */
export function moveColumn(
  columns: readonly ListColumn[],
  column: ListColumn,
  direction: "up" | "down",
): readonly ListColumn[] {
  const at = columns.indexOf(column);
  const to = direction === "up" ? at - 1 : at + 1;
  if (at < 0 || to < 0 || to >= columns.length) {
    return columns;
  }
  const moved = [...columns];
  moved.splice(at, 1);
  moved.splice(to, 0, column);
  return moved;
}
