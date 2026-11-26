import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import {
  readMoreCards,
  useBoardRead,
  type Board,
  type BoardAsk,
  type BoardCard,
} from "./board-data.ts";

/** What stands in one cell of the board: its cards so far, how many it holds, and where the next ones start. */
export interface CellView {
  readonly cards: readonly BoardCard[];
  readonly count: number;
  readonly next: string | null;
}

/** The name of a cell, from its column and its row (a board without rows has the row `null`). */
function cellKey(column: number, row: number | null): string {
  return `${String(column)}.${row === null ? "-" : String(row)}`;
}

/** The cards that *Show more* added to a cell, with the cursor after them. */
interface Added {
  readonly cards: readonly BoardCard[];
  readonly next: string | null;
}

/** What *Show more* read since the board was read: another read of the board takes it back, so no card shows twice. */
interface Further {
  readonly after: number;
  readonly cells: Readonly<Record<string, Added>>;
}

/** What the screen draws and does with a board that was read. */
export interface BoardModel {
  readonly board: Board;
  readonly cellOf: (column: number, row: number | null) => CellView;
  /** *Show more* of one cell. */
  readonly showMore: (column: number, row: number | null) => void;
  /** The cell whose next cards are on their way, by `cellKey`. */
  readonly pending: string | null;
  /** Why *Show more* failed, in words. */
  readonly problem: string | null;
  /** Every card shown, by the ID of its issue. */
  readonly cardOf: (issue: string) => BoardCard | undefined;
}

/** The cell of a board at a column and a row, with the cards *Show more* added. */
function viewOf(board: Board, shown: Readonly<Record<string, Added>>) {
  return (column: number, row: number | null): CellView => {
    const cell = board.cells.find((each) => each.column === column && each.row === row);
    const more = shown[cellKey(column, row)];
    return {
      cards: [...(cell?.cards ?? []), ...(more?.cards ?? [])],
      count: cell?.count ?? 0,
      next: more === undefined ? (cell?.next_cursor ?? null) : more.next,
    };
  };
}

/** The cards that *Show more* reads for the board of `ask`, and the cell that is waiting for them. */
function useFurther(ask: BoardAsk, readAt: number) {
  const [further, setFurther] = useState<Further>({ after: 0, cells: {} });
  const [pending, setPending] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const shown = further.after === readAt ? further.cells : {};
  const more = (key: string, cursor: string): void => {
    setPending(key);
    setProblem(null);
    readMoreCards(ask, cursor)
      .then((cell) => {
        setFurther({
          after: readAt,
          cells: {
            ...shown,
            [key]: {
              cards: [...(shown[key]?.cards ?? []), ...(cell?.cards ?? [])],
              next: cell?.next_cursor ?? null,
            },
          },
        });
      })
      .catch((error: unknown) => {
        setProblem(messageOf(error));
      })
      .finally(() => {
        setPending(null);
      });
  };
  return { shown, pending, problem, more };
}

/**
 * The board of a search and the cards that *Show more* added to its cells. Server data stays in the
 * query cache; the next pages of a cell are kept only until the board is read again.
 */
export function useBoard(ask: BoardAsk) {
  const read = useBoardRead(ask);
  const { shown, pending, problem, more } = useFurther(ask, read.dataUpdatedAt);
  const board = read.data;
  const model: BoardModel | undefined =
    board === undefined
      ? undefined
      : {
          board,
          pending,
          problem,
          cellOf: viewOf(board, shown),
          cardOf: (issue) =>
            [...board.cells, ...Object.values(shown)]
              .flatMap((cell) => cell.cards)
              .find((card) => card.issue.id === issue),
          showMore: (column, row) => {
            const key = cellKey(column, row);
            const cursor = viewOf(board, shown)(column, row).next;
            if (cursor !== null) {
              more(key, cursor);
            }
          },
        };
  return { read, model };
}

export { cellKey };
