import { listNeeds, type ListColumnSpec } from "@bugaboo/ui";

import { cellOf, listWord, staysColumns } from "../../shared/list-columns.ts";

/** What the fit counts: the room, the tick box, the widths a person gave and the longest ID drawn. */
export interface FitInput {
  /** The width the list has, in pixels; none before it is measured. */
  readonly room: number | undefined;
  /** The rows have a box to tick: a column of its own. */
  readonly ticks: boolean;
  /** The widths a person gave columns, by search word (dragged now, or kept in the view). */
  readonly widths: Readonly<Record<string, number>>;
  /** The characters of the longest readable ID drawn. */
  readonly keyChars: number;
}

/** The width the columns need as the list draws them. */
function needOf(words: readonly string[], { ticks, widths, keyChars }: FitInput): number {
  const specs: readonly ListColumnSpec[] = [
    ...(ticks ? [{ column: "select" } satisfies ListColumnSpec] : []),
    ...words.map((word) => ({ column: cellOf(word), width: widths[word] })),
  ];
  return listNeeds(specs, keyChars);
}

/**
 * The index of the column to drop next: the last column of a field from the right, except that a
 * column of an agent (*Agent*, *Agent state*) outranks *Changed*, which goes first while it is
 * still there. None where only the columns that stay are left.
 */
function nextToDrop(kept: readonly string[]): number {
  for (let at = kept.length - 1; at >= 0; at -= 1) {
    const word = kept[at] ?? "";
    if (staysColumns.has(word)) {
      continue;
    }
    const changed = kept.indexOf(listWord.changed);
    const forAgent = word === listWord.agent || word === listWord.agentState;
    return forAgent && changed >= 0 ? changed : at;
  }
  return -1;
}

/**
 * The columns that fit in the room of the list (UX §4.2): where the list has no room for all of
 * them as they are drawn (the widths a person gave counted, the ID as wide as it is), the columns of
 * fields are dropped from the right, one by one (*Changed* before the columns of an agent), until
 * it has; the status, the ID, the title and the priority stay. Where even those do not fit the list
 * scrolls. No room measured yet keeps every column.
 */
export function columnsThatFit(words: readonly string[], input: FitInput): readonly string[] {
  if (input.room === undefined) {
    return words;
  }
  const kept = [...words];
  for (
    let at = nextToDrop(kept);
    at >= 0 && needOf(kept, input) > input.room;
    at = nextToDrop(kept)
  ) {
    kept.splice(at, 1);
  }
  return kept;
}
