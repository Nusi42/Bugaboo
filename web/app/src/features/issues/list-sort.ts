import type { Direction, FieldKind, SearchLanguage } from "@bugaboo/wasm-search";

import { withSortBy } from "../../shared/sort-by.ts";

/** The way a list is ordered by one column: the arrow beside its name. */
export interface ListSort {
  /** The search word of the column. */
  readonly word: string;
  /** `up` runs from the least to the most (A to Z, oldest first), `down` the other way. */
  readonly arrow: "up" | "down";
}

/** The two directions of a kind of field, the first one a press starts with. */
const turns: Readonly<Record<FieldKind, readonly [Direction, Direction]>> = {
  text: ["a_to_z", "z_to_a"],
  number: ["lowest_first", "highest_first"],
  duration: ["lowest_first", "highest_first"],
  date: ["oldest_first", "newest_first"],
  choice: ["a_to_z", "z_to_a"],
  person: ["a_to_z", "z_to_a"],
  project: ["a_to_z", "z_to_a"],
  issue: ["a_to_z", "z_to_a"],
};

const ascending: ReadonlySet<Direction | null> = new Set([
  "a_to_z",
  "lowest_first",
  "oldest_first",
]);

/** The first key of `sort by` in the text, if it has one: what the column heads show. */
export function sortOf(language: SearchLanguage, text: string): ListSort | null {
  const key = language.parse(text).search?.sort[0];
  if (key === undefined) {
    return null;
  }
  return { word: key.field.word, arrow: ascending.has(key.direction) ? "up" : "down" };
}

/**
 * The text after a press on the name of a column: sorted by it from the least, and turned over on
 * the next press. The text is written by the printer of the search language (`withSortBy`), so the
 * search bar shows the order as it was typed.
 */
export function nextSort(
  language: SearchLanguage,
  text: string,
  word: string,
  kind: FieldKind,
): string {
  const [first, second] = turns[kind];
  const now = sortOf(language, text);
  const direction = now?.word === word && now.arrow === "up" ? second : first;
  return withSortBy(language, text, word, direction);
}
