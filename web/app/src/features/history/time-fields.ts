import type { Schema } from "@bugaboo/api-client";

import { ownValueText } from "../../shared/own-value-text.ts";

/** One value of a field as a sentence of history writes it beside the time: its names, its text or its typed value. */
function valueOf(side: Schema<"ChangeValueBody"> | null | undefined): string | undefined {
  if (side === null || side === undefined) {
    return undefined;
  }
  if (side.shown === "names") {
    return side.names.join(", ");
  }
  if (side.shown === "text") {
    return side.text;
  }
  return side.shown === "typed" ? ownValueText(side.kind, side.value) : undefined;
}

/**
 * The own fields of a time entry (*Activity*, *Cost center*) that the items after a booked-time item
 * of the same event carry, as the values they were given: the history says them in the sentence of
 * the time (*Ana booked 1h 30m · Development · 4100 Product development*) and not as sentences of
 * their own. A value that the reader may not see is not in the items, so it is not here.
 */
export function timeFieldValues(items: readonly Schema<"HistoryEntryItem">[]): readonly string[] {
  const values: string[] = [];
  for (const item of items) {
    const value = item.kind === "change" ? valueOf(item.change?.after) : undefined;
    if (value !== undefined && value !== "") {
      values.push(value);
    }
  }
  return values;
}
