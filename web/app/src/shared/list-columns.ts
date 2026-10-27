import type { SearchWord } from "@bugaboo/api-client";
import { listColumns, type ListColumn } from "@bugaboo/ui";

/** The cells of the list that are not a choice: the tick box and the buttons of the row. */
type FixedCell = "select" | "action";

/**
 * The search word of the field each cell shows. A saved search stores its columns by search word
 * (ADR-0034), so this is the one place that says which cell a word is drawn in. The words are the
 * generated ones, so a word that does not exist is a type error; so is a new cell in the theme file
 * until it is here. `id` is the Issue ID, the one word that is no field.
 */
const wordOfCell = {
  status: "status",
  key: "id",
  project: "project",
  type: "type",
  title: "title",
  labels: "labels",
  priority: "priority",
  person: "assignedTo",
  changed: "changed",
} as const satisfies Readonly<Record<Exclude<ListColumn, FixedCell>, SearchWord>>;

/** The cell a column is drawn in: one the list has a cell for, or a plain value (an own field). */
export type ColumnCell = Exclude<ListColumn, FixedCell> | "value";

/** The search words of the columns that cannot be taken away in *Customize → Columns* (UX §4.2). */
const keptWords: readonly string[] = [wordOfCell.key, wordOfCell.title];

/** The cells a person can choose from, in the order the project's list has them (ADR-0034 decision 6). */
const choiceCells = listColumns.filter(
  (cell): cell is Exclude<ColumnCell, "value"> => cell in wordOfCell,
);

/** The columns a project's list has by default, as search words, in the order of UX §4.2: a project's list has no *Project* column. */
export const projectColumns: readonly string[] = choiceCells
  .filter((cell) => cell !== "project")
  .map((cell) => wordOfCell[cell]);

/** The columns of the *Search* page by default: those of a project's list, and *Project* after the ID (UX §4.2). */
export const searchColumns: readonly string[] = projectColumns.flatMap((word) =>
  word === wordOfCell.key ? [word, wordOfCell.project] : [word],
);

const cellOfWord: ReadonlyMap<string, ColumnCell> = new Map(
  choiceCells.map((cell) => [wordOfCell[cell], cell]),
);

/** The cell a column with this search word is drawn in; a field without a cell of its own is a plain value. */
export function cellOf(word: string): ColumnCell {
  return cellOfWord.get(word) ?? "value";
}

/**
 * The columns after *Customize → Columns*, as search words: the ticked ones that the person can
 * read (a field of their own included, once each), in the order given, and the ID and the title
 * whatever was ticked (they come first when they were not ticked). A word that is no readable field
 * is left out.
 */
export function chooseColumns(
  ticked: readonly string[],
  readable: readonly string[],
): readonly string[] {
  const chosen = [...new Set(ticked)].filter(
    (word) => keptWords.includes(word) || readable.includes(word),
  );
  const missing = keptWords.filter((word) => !chosen.includes(word));
  return [...missing, ...chosen];
}

/** One column one place up or down; at an end it stays. */
export function moveColumn(
  columns: readonly string[],
  column: string,
  direction: "up" | "down",
): readonly string[] {
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
