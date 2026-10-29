import type { SearchWord } from "@bugaboo/api-client";
import { listColumns, type ListColumn } from "@bugaboo/ui";

/** The cells of the list that are not a choice: the tick box and the buttons of the row. */
type FixedCell = "select" | "action" | "value";

/**
 * The search words of the fields the list names besides those of its cells: the one place where
 * the list spells a word of a field, typed by the generated words (ODR).
 */
export const listWord = {
  id: "id",
  title: "title",
  description: "description",
  statusGroup: "statusGroup",
  assignedTo: "assignedTo",
  reportedBy: "reportedBy",
  created: "created",
  changed: "changed",
  closed: "closed",
  labels: "labels",
  project: "project",
  type: "type",
  status: "status",
  priority: "priority",
} as const satisfies Readonly<Record<string, SearchWord>>;

/**
 * The search word of the field each cell shows. A saved search stores its columns by search word
 * (ADR-0034), so this is the one place that says which cell a word is drawn in. The words are the
 * generated ones, so a word that does not exist is a type error; so is a new cell in the theme file
 * until it is here. `id` is the Issue ID, the one word that is no field.
 */
const wordOfCell = {
  status: listWord.status,
  key: listWord.id,
  project: listWord.project,
  type: listWord.type,
  title: listWord.title,
  labels: listWord.labels,
  priority: listWord.priority,
  person: listWord.assignedTo,
  changed: listWord.changed,
} as const satisfies Readonly<Record<Exclude<ListColumn, FixedCell>, SearchWord>>;

/** The cell a column is drawn in: one the list has a cell for, or a plain value (an own field). */
export type ColumnCell = Exclude<ListColumn, FixedCell> | "value";

/** The search words of the columns that cannot be taken away in *Customize → Columns* (UX §4.2). */
export const keptWords: readonly string[] = [wordOfCell.key, wordOfCell.title];

/** The cells a person can choose from, in the order the project's list has them (ADR-0034 decision 6). */
const choiceCells = listColumns.filter(
  (cell): cell is Exclude<ColumnCell, "value"> => cell in wordOfCell,
);

/**
 * The cells that are not in the default list, though a person can add them in *Customize*: *Project*
 * is the *Search* page's, and *Type* and *Labels* leave the title room on a window of 1280 (the title
 * is what people read: about a third of the window).
 */
const notByDefault: ReadonlySet<ColumnCell> = new Set(["project", "type", "labels"]);

/** The columns a project's list has by default, as search words, in the order of UX §4.2. */
export const projectColumns: readonly string[] = choiceCells
  .filter((cell) => !notByDefault.has(cell))
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

/** The words with no cell of their own that the list draws as text from the row (`valueOf`). */
const wordsWithText: ReadonlySet<string> = new Set([
  listWord.created,
  listWord.closed,
  listWord.reportedBy,
  listWord.statusGroup,
]);

/** Whether a column or a group is a field of the organization: a word with no cell of its own and no text drawn from the row. */
export function isOwnField(word: string): boolean {
  return cellOf(word) === "value" && !wordsWithText.has(word);
}

/** The words that cannot be grouped by: a long text and the Issue ID have no groups. */
export const notGroupable: ReadonlySet<string> = new Set([listWord.description, listWord.id]);

/**
 * The columns the list keeps while an issue is open in the side panel beside it (UX §4.2): the
 * list has the room that is left, so only the status, the ID, the title and the priority stay.
 */
export const besidePanelColumns: ReadonlySet<string> = new Set([
  listWord.status,
  listWord.id,
  listWord.title,
  listWord.priority,
]);

/** The columns a phone draws in each card (UX section 4.2): columns are not offered there. */
export const phoneColumns: readonly string[] = [
  listWord.status,
  listWord.id,
  listWord.priority,
  listWord.title,
  listWord.assignedTo,
  listWord.changed,
];

/** The kinds of field whose value an issue carries only as an ID: no cell shows them yet (M5-F84). */
const kindsOfIds: ReadonlySet<string> = new Set(["person", "project", "issue"]);

/** What the list needs to know of a field the person may read. */
export interface ColumnField {
  readonly kind: string;
  /** The words of its choices; none for a field that has no list of them. */
  readonly choices: readonly string[];
}

/**
 * Whether a column can be drawn: the field is one the person may read, and it is not a field of the
 * organization whose value is an ID, which the list cannot name yet (person, people, issue, project,
 * and a group of people, which the search language lists as a choice field without choices). The fields
 * with a cell of their own (*Assigned to*, *Project*) are fine. A column that cannot be drawn is left
 * out quietly, wherever it was named: the address, a saved search or *Customize*.
 */
export function isDrawnColumn(word: string, field?: ColumnField): boolean {
  // A word with a cell of its own is a field every issue has (the Issue ID is no field at all).
  if (cellOf(word) !== "value") {
    return true;
  }
  if (wordsWithText.has(word)) {
    return true;
  }
  // A long text has no value a cell can show.
  if (field === undefined || word === listWord.description) {
    return false;
  }
  return !kindsOfIds.has(field.kind) && !(field.kind === "choice" && field.choices.length === 0);
}
