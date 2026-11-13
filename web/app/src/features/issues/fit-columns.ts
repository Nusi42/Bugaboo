import { listNeeds, type ListColumnSpec } from "@bugaboo/ui";

import { cellOf, listWord, staysColumns } from "../../shared/list-columns.ts";
import { isAgentWord } from "./agent-value.ts";

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
 * The columns that stay longest, in the order they give way where even they do not fit: the
 * priority, then the status, then the ID. The title never gives way.
 */
const lastToGo = [listWord.priority, listWord.status, listWord.id] as const;

/**
 * The index of the column to drop next: the last column of a field from the right, except that a
 * column of an agent (*Agent*, *Agent state*) outranks *Changed*, which goes first while it is
 * still there; where only the columns that stay are left, the first of {@link lastToGo} that is
 * there. None where only the title is left.
 */
function nextToDrop(kept: readonly string[]): number {
  for (let at = kept.length - 1; at >= 0; at -= 1) {
    const word = kept[at] ?? "";
    if (staysColumns.has(word)) {
      continue;
    }
    const changed = kept.indexOf(listWord.changed);
    return isAgentWord(word) && changed >= 0 ? changed : at;
  }
  const last = lastToGo.find((word) => kept.includes(word));
  return last === undefined ? -1 : kept.indexOf(last);
}

/**
 * The columns that fit in the room of the list (UX §4.2): where the list has no room for all of
 * them as they are drawn (the widths a person gave counted, the ID as wide as it is), the columns of
 * fields are dropped from the right, one by one (*Changed* before the columns of an agent), until
 * it has; the status, the ID, the title and the priority stay longest. Where even those do not fit,
 * the priority, the status and the ID give way in that order, so the list does not scroll sideways;
 * only the title alone, at its least width, can still be wider than the room. The room is what the
 * rows have, without the bar they scroll with. No room measured yet keeps every column.
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
