import { t } from "@bugaboo/messages";
import type { MenuAction } from "@bugaboo/ui";
import type { SearchLanguage } from "@bugaboo/wasm-search";

import { listWord } from "../../shared/list-columns.ts";
import { historyStep, withSortBy, type HistoryStep } from "../search/index.ts";

/** The fields a column's menu offers to order the cards by: the ones a board is read by. */
const orderWords = [listWord.priority, listWord.dueDate, listWord.changed] as const;

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
 * the sort of the search, and each entry sorts the search by a field, as a click on a column of the list does.
 */
export function orderActions({ language, text, onText, names }: OrderInput): readonly MenuAction[] {
  return orderWords.map((word) => ({
    id: word,
    label: t("board.sort-by", { field: names[word] ?? word }),
    onAction: () => {
      onText(withSortBy(language, text, word, null), historyStep.push);
    },
  }));
}
