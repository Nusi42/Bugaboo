import type { SearchLanguage } from "@bugaboo/wasm-search";
import { createContext, useContext } from "react";

import type { SearchState } from "../../shared/search-address.ts";
import type { FoundSearch } from "./found-issues.ts";
import type { IssueShow } from "./issue-show.ts";
import type { OwnColumns } from "./own-columns.ts";
import type { ListShown } from "./use-list-shown.ts";
import type { IssueSelection } from "./use-selection.ts";

/** What every row and every group of one list shares. */
export interface ListContextValue {
  /** What the head cells of the list and the cells under them name each other by (one for each list on the page). */
  readonly listId: string;
  /** The search words of the columns, in order. */
  readonly words: readonly string[];
  /** A phone draws a card for each issue and has no columns of its own. */
  readonly phone: boolean;
  /** The rows have a box to tick (the list of a project; the changes of many issues are made in one project). */
  readonly ticks: boolean;
  readonly openIn: IssueShow;
  /** What the address of the list says: the links to the side panel keep it. */
  readonly state: SearchState | undefined;
  /** The issue open next to the list, by its issue ID. */
  readonly open: string | undefined;
  readonly selection: IssueSelection;
  /** The project whose address an issue has where its own project is not known. */
  readonly project: string | undefined;
  readonly now: number;
  readonly own: OwnColumns;
  readonly search: FoundSearch;
  readonly language: SearchLanguage;
  /** The field the list is grouped by. */
  readonly groupBy: string | null;
  /** Why the search was refused, so the list has no rows and no head; none while it has a list. */
  readonly failure: Error | null;
  /** Takes every filter out of the search (*Remove all filters*). */
  readonly onClear: () => void;
  /** A group (or the whole list) tells which issues it shows now. */
  readonly report: ListShown["report"];
}

export const ListContext = createContext<ListContextValue | null>(null);

/** What the list around a row shares with it. */
export function useListContext(): ListContextValue {
  const value = useContext(ListContext);
  if (value === null) {
    throw new Error("A row of the issue list is drawn outside the list.");
  }
  return value;
}
