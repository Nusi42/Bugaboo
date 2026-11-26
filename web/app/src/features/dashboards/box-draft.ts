import {
  kindsWithShape,
  kindsWithSpan,
  kindsWithoutSearch,
  spansWithoutSearch,
  type Schema,
} from "@bugaboo/api-client";

import { chartShape, timeSpan, type AskedBox } from "./dashboards.ts";

/** Whether a word is one of a list of words the server names. */
function among(words: readonly string[], word: string): boolean {
  return words.includes(word);
}

/** The word of no field: a chart of one number. */
export const noField = "";

/** Whether a box of this kind, with these options, reads a search, by the rules of the server (`BoxSpec::reads_a_search`). */
export function readsSearch(kind: Schema<"BoxKind">, span: Schema<"TimeSpan">): boolean {
  return (
    !among(kindsWithoutSearch, kind) &&
    !(among(kindsWithSpan, kind) && among(spansWithoutSearch, span))
  );
}

/** Whether a box of this kind shows its totals in a shape (bars by a field, or one number). */
export function hasShape(kind: Schema<"BoxKind">): boolean {
  return among(kindsWithShape, kind);
}

/** Whether a box of this kind says whose time it adds up. */
export function hasSpan(kind: Schema<"BoxKind">): boolean {
  return among(kindsWithSpan, kind);
}

/**
 * The box the form asks for: the options of its kind and none of another (the server refuses those), by the lists of
 * the server; a chart takes a field only as bars, so no field is one number.
 */
export function askedOf(start: AskedBox, draft: Draft): AskedBox {
  const numbers = hasShape(draft.kind);
  const bars = numbers && draft.by !== noField;
  const reads = readsSearch(draft.kind, draft.span);
  return {
    ...start,
    kind: draft.kind,
    title: draft.title.trim(),
    by: bars ? draft.by : null,
    shape: numbers ? (bars ? chartShape.bars : chartShape.number) : null,
    span: hasSpan(draft.kind) ? draft.span : null,
    search_text: reads && (start.saved_search ?? null) === null ? draft.text : null,
    saved_search: reads ? (start.saved_search ?? null) : null,
  };
}

/** What the form holds while it is filled in. */
export interface Draft {
  readonly kind: Schema<"BoxKind">;
  readonly title: string;
  readonly text: string;
  readonly by: string;
  readonly span: Schema<"TimeSpan">;
}

/** The draft of a box as it starts: its kind, title, search and options. */
export function draftOf(box: AskedBox): Draft {
  return {
    kind: box.kind,
    title: box.title,
    text: box.search_text ?? "",
    by: box.by ?? noField,
    span: box.span ?? timeSpan.search,
  };
}
