import { t } from "@bugaboo/messages";
import { part, partOf, Tabs } from "@bugaboo/ui";
import { useState } from "react";

import { BoardCell } from "./BoardCell.tsx";
import { BoardRow } from "./BoardRow.tsx";
import { columnName } from "./card-moves.ts";
import type { OwnOf } from "./CardFields.tsx";
import type { BoardGridProps } from "./BoardGrid.tsx";

/**
 * The board on a phone (UX §4.6, M10-Q1): one column at a time, the names of the columns as tabs with
 * their counts. Nothing is dragged: every card has *Move to …*.
 */
export function PhoneBoard({ model, own, rowName }: BoardGridProps & { readonly own: OwnOf }) {
  const { columns, rows } = model.board;
  const [chosen, setChosen] = useState(0);
  const open = Math.min(chosen, columns.length - 1);
  const tabs = columns.map((column, at) => ({
    id: String(at),
    label: t("board.tab", { name: columnName(column), count: column.count }),
  }));
  return (
    <div {...partOf(part.boardPhone)}>
      <Tabs
        label={t("board.tabs")}
        tabs={tabs}
        selected={String(open)}
        onSelect={(id) => {
          setChosen(Number(id));
        }}
      >
        {(id) => {
          const at = Number(id);
          const column = columns[at];
          if (column === undefined) {
            return null;
          }
          const name = columnName(column);
          return rows.length === 0 ? (
            <BoardCell model={model} name={name} at={{ column: at, row: null }} own={own} />
          ) : (
            rows.map((row, place) => (
              <BoardRow key={JSON.stringify(row.key)} name={rowName(row)} count={row.issues}>
                <BoardCell model={model} name={name} at={{ column: at, row: place }} own={own} />
              </BoardRow>
            ))
          );
        }}
      </Tabs>
    </div>
  );
}
