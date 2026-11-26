import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";

import { ColumnHead } from "./ColumnHead.tsx";
import { columnId } from "./card-moves.ts";
import type { OwnOf } from "./CardFields.tsx";
import { BoardLane } from "./BoardLane.tsx";
import { BoardRow } from "./BoardRow.tsx";
import type { BoardModel } from "./use-board.ts";

export interface BoardGridProps {
  readonly model: BoardModel;
  readonly own: OwnOf;
  /** What a row is called. */
  readonly rowName: (row: BoardModel["board"]["rows"][number]) => string;
}

/**
 * The board on a computer or a tablet (UX §4.6): the heads of the columns, then the cards, in one
 * box that scrolls sideways. Without rows the columns stand under their heads; with rows each row
 * has a header that folds it and its own cells under the same heads.
 */
export function BoardGrid({ model, own, rowName }: BoardGridProps) {
  const { columns, rows } = model.board;
  return (
    <div {...partOf(part.boardScroll)}>
      <div {...partOf(part.boardHead)}>
        {columns.map((column, at) => (
          <ColumnHead key={columnId(column)} column={column} at={at} />
        ))}
      </div>
      {rows.length === 0 ? (
        <BoardLane model={model} row={null} own={own} />
      ) : (
        rows.map((row, at) => (
          <BoardRow key={JSON.stringify(row.key)} name={rowName(row)} count={row.issues}>
            <BoardLane model={model} row={at} own={own} />
          </BoardRow>
        ))
      )}
      {model.board.rows_truncated ? (
        <p {...partOf(part.boardNote)}>{t("board.rows-truncated", { count: rows.length })}</p>
      ) : null}
    </div>
  );
}
