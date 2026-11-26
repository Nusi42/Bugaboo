import { part, partOf } from "@bugaboo/ui";

import type { BoardColumn } from "./board-data.ts";
import { BoardCell } from "./BoardCell.tsx";
import { columnId, columnName } from "./card-moves.ts";
import type { OwnOf } from "./CardFields.tsx";
import type { BoardModel } from "./use-board.ts";

export interface BoardLaneProps {
  readonly model: BoardModel;
  /** The row the cells belong to, by its place; `null` on a board without rows. */
  readonly row: number | null;
  readonly own: OwnOf;
}

/** The cells of one row (or of the whole board without rows), a cell for each column, side by side. */
export function BoardLane({ model, row, own }: BoardLaneProps) {
  return (
    <div {...partOf(part.boardLane)}>
      {model.board.columns.map((column: BoardColumn, at) => (
        <BoardCell
          key={columnId(column)}
          model={model}
          name={columnName(column)}
          at={{ column: at, row }}
          own={own}
        />
      ))}
    </div>
  );
}
