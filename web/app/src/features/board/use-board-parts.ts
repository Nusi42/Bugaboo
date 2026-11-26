import { t } from "@bugaboo/messages";
import { useId } from "react";

import type { BoardOptions } from "../../shared/list-view.ts";
import type { OwnColumns } from "../issues/index.ts";
import { useHolding, type BoardState } from "./holding.ts";
import { orderActions, type OrderInput } from "./order-actions.ts";
import type { BoardModel } from "./use-board.ts";
import { useCardMoves } from "./use-card-moves.ts";
import { useRowNames } from "./use-row-names.ts";

/** What the moves, the card in hand and the names of the rows are made from. */
export interface BoardPartsInput {
  readonly model: BoardModel;
  readonly options: BoardOptions;
  readonly search: { readonly text: string; readonly within: string | undefined };
  readonly own: OwnColumns;
  readonly order: OrderInput;
  readonly phone: boolean;
}

/** What the parts of a board that was read share: the card in hand, the moves, and the words around them. */
export function useBoardParts({ model, options, search, own, order, phone }: BoardPartsInput) {
  const { columns } = model.board;
  const field = order.names[options.columns ?? "status"] ?? t("issue-field.status");
  const moves = useCardMoves({
    columns,
    field: options.columns === null ? undefined : own[options.columns],
  });
  const holding = useHolding({ columns, field, moves, cardOf: model.cardOf });
  const rowName = useRowNames({ search, rows: options.rows, own, names: order.names });
  const keysId = useId();
  const state: BoardState = {
    holding,
    moves,
    columns,
    phone,
    field,
    keysId,
    order: orderActions(order),
  };
  return { state, rowName };
}
