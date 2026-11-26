import { t } from "@bugaboo/messages";
import { part, partOf, ShowMore } from "@bugaboo/ui";
import { useRef } from "react";

import { dragKinds } from "../../shared/drag-kinds.ts";
import { useDropOf } from "../../shared/drop-of.ts";
import { BoardCard } from "./BoardCard.tsx";
import type { OwnOf } from "./CardFields.tsx";
import { heldBy, useBoardState } from "./holding.ts";
import { cellKey, type BoardModel } from "./use-board.ts";

export interface BoardCellProps {
  readonly model: BoardModel;
  /** What the column is called. */
  readonly name: string;
  /** The column and the row of the cell, by their places; the row is `null` on a board without rows. */
  readonly at: { readonly column: number; readonly row: number | null };
  readonly own: OwnOf;
}

/**
 * The cards of one column in one row (UX §4.6): where a card can be dropped, unless the column fades
 * for it, with the reason over its cards while it does. A long column shows its first page and *Show
 * more*. A drop in the column the card comes from does nothing: the order of the cards is the sort
 * of the search (M10-Q3).
 */
export function BoardCell({ model, name, at, own }: BoardCellProps) {
  const { holding, phone } = useBoardState();
  const ref = useRef<HTMLUListElement>(null);
  const offer = holding.offerOf(at.column);
  const drop = useDropOf(dragKinds.card.kind, ref, !phone && !offer.faded, () => {
    holding.dropOn(at.column);
  });
  const cell = model.cellOf(at.column, at.row);
  const target = holding.held?.by === heldBy.keys && holding.held.target === at.column;
  return (
    <div {...partOf(part.boardCell, { variants: [phone && "phone"] })}>
      <ul
        ref={ref}
        aria-label={t("board.cards-of", { column: name })}
        {...drop.dropProps}
        {...partOf(part.boardColumn, {
          states: [offer.faded && "faded", target && "target", drop.isDropTarget && "over"],
        })}
      >
        {cell.cards.map((card) => (
          <BoardCard key={card.issue.id} card={card} column={at.column} own={own} />
        ))}
        {cell.cards.length === 0 ? (
          <li {...partOf(part.boardColumnEmpty)}>{t("board.empty-column")}</li>
        ) : null}
        {cell.next === null ? null : (
          <li {...partOf(part.boardMore)}>
            <ShowMore
              label={t("board.show-more")}
              pending={model.pending === cellKey(at.column, at.row)}
              onPress={() => {
                model.showMore(at.column, at.row);
              }}
            />
          </li>
        )}
      </ul>
      {holding.held !== null && offer.faded && offer.reason !== undefined ? (
        <div {...partOf(part.boardColumnReason)}>{offer.reason}</div>
      ) : null}
    </div>
  );
}
