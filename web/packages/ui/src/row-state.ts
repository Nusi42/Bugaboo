import { partWords } from "./tokens.ts";
import type { PriorityWord, StatusGroup } from "./status.ts";

type WordsOf<Row extends keyof typeof partWords> = (typeof partWords)[Row]["state"][number];

/**
 * The words of the state that every row can carry (the list row, the issue row and the bar of the
 * timeline): the tint of a status group and the edge of a priority. The theme file styles them.
 */
export type RowStateWord = Extract<
  Extract<WordsOf<"list-row">, WordsOf<"issue-row">>,
  WordsOf<"timeline-bar">
>;

/** Every status group has its tint; the rows of a normal or low priority have no edge. */
const groupWord: Readonly<Record<StatusGroup, RowStateWord | undefined>> = {
  "not-started": "group-not-started",
  "in-progress": "group-in-progress",
  done: "group-done",
};

const priorityWord: Readonly<Record<PriorityWord, RowStateWord | undefined>> = {
  low: undefined,
  normal: undefined,
  high: "priority-high",
  critical: "priority-critical",
};

export interface RowStateInput {
  readonly group: StatusGroup;
  readonly priority: PriorityWord;
}

/** `states: rowStates({ group, priority })` on any row: its tint and its edge (R4). */
export function rowStates({ group, priority }: RowStateInput): readonly RowStateWord[] {
  return [groupWord[group], priorityWord[priority]].filter((word) => word !== undefined);
}
