import { partWords } from "./tokens.ts";

/** A column of the issue list: the theme file names them as the variants of a cell. */
export type ListColumn = (typeof partWords)["list-row.cell"]["variant"][number];

/**
 * Where each column stands, from the start (UX §4.2, R9): tick box, status, ID, type, title,
 * labels, priority, assigned person, changed. A column the theme file knows and this list does not
 * is a type error.
 */
const position = {
  select: 0,
  status: 1,
  key: 2,
  type: 3,
  title: 4,
  labels: 5,
  priority: 6,
  person: 7,
  changed: 8,
} as const satisfies Readonly<Record<ListColumn, number>>;

/** The columns in the order they stand, for the header and for every row. */
export const listColumns: readonly ListColumn[] = partWords["list-row.cell"].variant.toSorted(
  (first, second) => position[first] - position[second],
);

/** The columns that hold only an icon: the header names them for screen readers, not for the eye. */
export const iconColumns: readonly ListColumn[] = ["select", "status", "type"];
