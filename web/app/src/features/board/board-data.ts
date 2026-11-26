import { dataOf, type Schema } from "@bugaboo/api-client";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import type { BoardOptions } from "../../shared/list-view.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A board as the server sends it: columns, rows and the first cards of every cell. */
export type Board = Schema<"BoardPageBody">;
export type BoardCard = Schema<"BoardCardBody">;
export type BoardCell = Schema<"BoardCellBody">;
export type BoardColumn = Schema<"BoardColumnBody">;
export type BoardRow = Schema<"BoardRowBody">;
/** Where a card may go, or why not (`may_move` of the workflow, judged by the server). */
export type CardMove = Schema<"CardMoveBody">;

/** How many cards one cell holds at first (1 to 50): *Show more* reads the next ones. */
export const cardsPerCell = 20;

/** What a board is of: a search, where it looks, and the options of its view. */
export interface BoardAsk {
  /** The canonical text of the search. */
  readonly text: string;
  /** The ID of the project it looks in; none looks in every project the person sees. */
  readonly within: string | undefined;
  readonly options: BoardOptions;
}

/** The body of a request for the board: the cursor of one cell for *Show more*, none for the whole board. */
function bodyOf(ask: BoardAsk, more?: string) {
  const { card_limits: limits, columns, rows } = ask.options;
  return {
    search_text: ask.text,
    ...(ask.within === undefined ? {} : { project: ask.within }),
    board: {
      columns,
      rows,
      card_limits: (limits ?? []).map(({ column, limit }) => ({ column, limit })),
    },
    limit: cardsPerCell,
    ...(more === undefined ? {} : { more }),
  };
}

/** The board of a search, read again after any change of an issue (its key starts with the lists'). */
export function useBoardRead(ask: BoardAsk) {
  return useQuery({
    queryKey: queryKeys.search.board(ask.text, ask.within ?? "", JSON.stringify(ask.options)),
    placeholderData: keepPreviousData,
    queryFn: async (): Promise<Board> =>
      dataOf(await api.client.POST("/api/v1/board", { body: bodyOf(ask) })),
  });
}

/** The next cards of the cell whose cursor this is (*Show more*), with the cursor after them. */
export async function readMoreCards(ask: BoardAsk, cursor: string): Promise<BoardCell | undefined> {
  const page: Board = dataOf(await api.client.POST("/api/v1/board", { body: bodyOf(ask, cursor) }));
  return page.cells[0];
}

/** Reads every board again: a card moved, or the person asked for it (*Refresh*). */
export function refreshBoards(): Promise<void> {
  return queryClient.invalidateQueries({ queryKey: queryKeys.search.board() });
}
