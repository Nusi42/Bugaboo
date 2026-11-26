import { t } from "@bugaboo/messages";
import type { MenuAction } from "@bugaboo/ui";
import type { SearchLanguage } from "@bugaboo/wasm-search";

import { listWord } from "../../shared/list-columns.ts";
import { sortOf } from "../issues/index.ts";
import { historyStep, withSortBy, type HistoryStep } from "../search/index.ts";

/** The fields a column's menu offers to order the cards by: the ones a board is read by. */
const orderWords: readonly string[] = [listWord.priority, listWord.dueDate, listWord.changed];

/** What the menu of a column needs to change the sort of the search. */
export interface OrderInput {
  readonly language: SearchLanguage;
  /** The canonical text of the search. */
  readonly text: string;
  readonly onText: (text: string, step: HistoryStep) => void;
  /** The names of the fields the person may read, by their words. */
  readonly names: Readonly<Record<string, string>>;
}

/**
 * The entries of the menu of a column (M10-Q3): the cards are not put in an order by hand, they follow
 * the sort of the search, and each entry sorts the search, so the whole board, by a field, as a click on
 * a column of the list does. The entry of the sort that holds now says so; a sort by another field
 * (typed in the search bar) is named first.
 */
export function orderActions({ language, text, onText, names }: OrderInput): readonly MenuAction[] {
  const now = sortOf(language, text)?.word;
  const words =
    now === undefined || orderWords.some((word) => word === now)
      ? orderWords
      : [now, ...orderWords];
  return words.map((word) => ({
    id: word,
    label: t(word === now ? "board.sorted-by-now" : "board.sort-by", {
      field: names[word] ?? word,
    }),
    onAction: () => {
      onText(withSortBy(language, text, word, null), historyStep.push);
    },
  }));
}
