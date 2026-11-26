import type { Schema } from "@bugaboo/api-client";

import { boxKind, chartShape, timeSpan, type AskedBox } from "./dashboards.ts";

/** The word of no field: a chart of one number. */
export const noField = "";

/** Whether a box of this kind, with these options, reads a search: every kind but *My timer* and the own week. */
export function readsSearch(kind: Schema<"BoxKind">, span: Schema<"TimeSpan">): boolean {
  return kind !== boxKind.myTimer && !(kind === boxKind.timeTotals && span === timeSpan.myWeek);
}

/** The box the form asks for: the options of its kind and none of another (the server refuses those). */
export function askedOf(start: AskedBox, draft: Draft): AskedBox {
  const numbers = draft.kind === boxKind.numbersAndChart;
  const bars = numbers && draft.by !== noField;
  const reads = readsSearch(draft.kind, draft.span);
  return {
    ...start,
    kind: draft.kind,
    title: draft.title.trim(),
    by: bars ? draft.by : null,
    shape: numbers ? (bars ? chartShape.bars : chartShape.number) : null,
    span: draft.kind === boxKind.timeTotals ? draft.span : null,
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
