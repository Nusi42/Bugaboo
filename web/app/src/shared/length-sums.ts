import { sumWords } from "@bugaboo/api-client";
import type { MessageKey } from "@bugaboo/messages";

import { listWord } from "./list-columns.ts";

/**
 * The three lengths a total adds up (QRY-08): the word of each sum, the search word of its column in a list, and its name.
 * The totals row of a grouped list and the box *Time totals* both read this one list.
 */
export const lengthSums = [
  { sum: sumWords.estimated, column: listWord.estimatedTime, name: "issue-field.estimated-time" },
  { sum: sumWords.timeLeft, column: listWord.timeLeft, name: "issue-field.time-left" },
  { sum: sumWords.timeSpent, column: listWord.timeSpent, name: "issue-field.time-spent" },
] as const satisfies readonly {
  readonly sum: string;
  readonly column: string;
  readonly name: MessageKey;
}[];
