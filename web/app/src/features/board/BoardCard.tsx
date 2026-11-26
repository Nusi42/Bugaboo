import { part, partOf } from "@bugaboo/ui";
import { useRef } from "react";

import type { BoardCard as Card } from "./board-data.ts";
import { cardCells } from "./card-cells.ts";
import { issueOf } from "./card-moves.ts";
import { CardFields, type OwnOf } from "./CardFields.tsx";
import { useBoardState } from "./holding.ts";
import { MoveTo } from "./MoveTo.tsx";
import { useCardDrag } from "./use-card-drag.ts";
import { useCardKeys } from "./use-card-keys.ts";

export interface BoardCardProps {
  readonly card: Card;
  /** The column the card stands in, by its place. */
  readonly column: number;
  readonly own: OwnOf;
}

/**
 * A card (UX §4.6, M10-Q4): its Issue ID, its title, and the fields of the layout *Card*, tinted by
 * its status group and edged by its priority as the rows of a list are (R4). A pointer takes it
 * anywhere; the keys of its title (*Space*, the arrows, *Enter*, *Esc*) and its *Move to …* do the same
 * (TEST-A11Y). A card that is on its way to another column shows it, and stays until the server
 * has answered.
 */
export function BoardCard({ card, column, own }: BoardCardProps) {
  const state = useBoardState();
  const { look, dragProps } = useCardDrag(card, column, state);
  const ref = useRef<HTMLLIElement>(null);
  useCardKeys(ref, { card, column, state });
  const cells = cardCells(card);
  return (
    <li
      ref={ref}
      data-issue={issueOf(card)}
      aria-describedby={state.keysId}
      {...look}
      {...dragProps}
    >
      <div {...partOf(part.boardCardLine)}>
        {cells.type}
        {cells.key}
        <span {...partOf(part.boardCardMove)}>
          <MoveTo card={card} column={column} />
        </span>
      </div>
      <span {...partOf(part.boardCardTitle)}>{cells.title}</span>
      <CardFields card={card} own={own} />
    </li>
  );
}
