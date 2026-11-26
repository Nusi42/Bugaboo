import type { Schema } from "@bugaboo/api-client";

/** The options of a board: the field of its columns, the field of its rows and the card limits. */
export type BoardOptions = Schema<"BoardOptionsBody">;

/** The board a saved search has when it says nothing of one: a column for each status, no rows, no card limits. */
export const builtInBoard: BoardOptions = { columns: null, rows: null, card_limits: [] };

/** The board options as a saved search keeps them: left out while they are the starting ones. */
export function boardOf(board: BoardOptions): Pick<Schema<"PresentationRef">, "board"> {
  const usual =
    board.columns === null && board.rows === null && (board.card_limits ?? []).length === 0;
  return usual ? {} : { board };
}
