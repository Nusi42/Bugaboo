import { t } from "@bugaboo/messages";
import { ActionMenu, MoreButton, part, partOf, StatusPill } from "@bugaboo/ui";
import { VisuallyHidden } from "react-aria-components";

import type { BoardColumn } from "./board-data.ts";
import { columnName } from "./card-moves.ts";
import { heldBy, useBoardState } from "./holding.ts";

/** What a column says of its cards: how many, and "4 of 3" where it has a card limit. */
function countText(column: BoardColumn): string {
  const count = column.count;
  return column.limit === null
    ? t("board.count", { count })
    : t("board.count-of-limit", { count, limit: column.limit });
}

/**
 * The head of a column (UX §4.6): its name and how many cards it holds, with its card limit when it
 * has one ("4 of 3"); over the limit the title turns orange and nothing is refused (M10-Q2). While a
 * card is in hand a column it cannot go to fades. The menu of the column names the order of its cards.
 */
export function ColumnHead({ column, at }: { readonly column: BoardColumn; readonly at: number }) {
  const { holding, order } = useBoardState();
  const name = columnName(column);
  const target = holding.held?.by === heldBy.keys && holding.held.target === at;
  const states = ["over" as const].filter(() => column.over);
  return (
    <div
      {...partOf(part.boardHeadCell, {
        states: [holding.offerOf(at).faded && "faded", target && "target"],
      })}
    >
      <span {...partOf(part.boardHeadTitle, { states })}>
        {column.status_group === null ? (
          name
        ) : (
          <StatusPill group={column.status_group} label={name} />
        )}
        <span {...partOf(part.boardHeadCount, { states })}>
          <span aria-hidden={column.over}>{countText(column)}</span>
          {column.over && column.limit !== null ? (
            <VisuallyHidden>
              {t("board.count-over", { count: column.count, limit: column.limit })}
            </VisuallyHidden>
          ) : null}
        </span>
      </span>
      <ActionMenu
        label={t("board.order-menu", { column: name })}
        actions={order}
        trigger={MoreButton}
        cancelLabel={t("common.cancel")}
      />
    </div>
  );
}
