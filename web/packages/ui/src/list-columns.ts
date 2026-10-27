import { partWords } from "./tokens.ts";

/** A column of the issue list: the theme file names them as the variants of a cell. */
export type ListColumn = (typeof partWords)["list-row.cell"]["variant"][number];

/**
 * Where each column stands, from the start (UX §4.2, R9): tick box, status, ID, project, type, title,
 * labels, priority, assigned person, changed, and the buttons of the row at the end. A column the theme file knows and this list does not
 * is a type error.
 */
const position = {
  select: 0,
  status: 1,
  key: 2,
  project: 3,
  type: 4,
  title: 5,
  labels: 6,
  priority: 7,
  person: 8,
  value: 9,
  changed: 10,
  action: 11,
} as const satisfies Readonly<Record<ListColumn, number>>;

/** The columns in the order they stand, for the header and for every row. */
export const listColumns: readonly ListColumn[] = partWords["list-row.cell"].variant.toSorted(
  (first, second) => position[first] - position[second],
);

/** The columns that hold only an icon: the header names them for screen readers, not for the eye. */
export const iconColumns: readonly ListColumn[] = ["select", "status", "type"];
