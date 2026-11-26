import { part, partOf, rowStates } from "@bugaboo/ui";

import { dragKinds } from "../../shared/drag-kinds.ts";
import { useDragOf } from "../../shared/drop-of.ts";
import type { BoardCard } from "./board-data.ts";
import { issueOf } from "./card-moves.ts";
import { heldBy, type BoardState } from "./holding.ts";

/**
 * What makes a card draggable by the pointer and what it looks like while it is dragged, picked up
 * by keys or on its way to another column: the attributes of its element (its part, with the tint of
 * its status group and the edge of its priority) and the props of its drag.
 */
export function useCardDrag(card: BoardCard, column: number, state: BoardState) {
  const { holding, moves, phone } = state;
  const id = issueOf(card);
  const drag = useDragOf(dragKinds.card.kind, id, {
    enabled: !phone,
    onStart: () => {
      holding.take(card, column, heldBy.pointer);
    },
    onEnd: holding.release,
  });
  const picked = holding.held?.by === heldBy.keys && holding.held.issue === id;
  const group = card.issue.status?.status_group ?? "not-started";
  return {
    dragProps: drag.dragProps,
    group,
    look: partOf(part.boardCard, {
      states: [
        ...rowStates({ group, priority: card.issue.priority ?? "normal" }),
        drag.isDragging && "dragging",
        moves.moving.includes(id) && "moving",
        picked && "picked",
        phone && "locked",
      ],
    }),
  };
}
