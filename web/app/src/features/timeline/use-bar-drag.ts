import { useState } from "react";

import { usePointerDrag, type DragHandlers } from "../../shared/pointer-drag.ts";
import type { Move } from "./moves.ts";

export type { DragHandlers } from "../../shared/pointer-drag.ts";

/** A drag that is under way: what it moves and by how many days so far. */
export interface Dragging {
  readonly move: Move;
  readonly days: number;
}

/** What the drag of a bar is told: the width of a day, and what happens at its end. */
export interface BarDragInput {
  readonly dayPixels: number;
  /** Whether the bar may be dragged at all: a bar the person may not change is only pressed. */
  readonly mayDrag: boolean;
  readonly onEnd: (move: Move, days: number) => void;
  readonly onPress: () => void;
}

/**
 * Dragging a bar with a pointer (PLAN-03, UX §4.7): while it is dragged it only moves on screen,
 * by whole days of the width the chart draws; when it is let go, `onEnd` gets the days, and a press
 * that did not move is a press (`onPress`). Only the main button drags, and a bar that may not be
 * changed starts no drag (`shared/pointer-drag.ts`). Every drag has a key too (`bar-keys.ts`).
 */
export function useBarDrag({ dayPixels, mayDrag, onEnd, onPress }: BarDragInput): {
  readonly dragging: Dragging | null;
  readonly handlers: (move: Move) => DragHandlers;
} {
  const [dragging, setDragging] = useState<Dragging | null>(null);
  const daysOf = (x: number): number => (dayPixels <= 0 ? 0 : Math.round(x / dayPixels));
  const { handlers } = usePointerDrag<Move>({
    drags: () => mayDrag,
    onStart: (move) => {
      setDragging({ move, days: 0 });
    },
    onMove: (move, distance) => {
      setDragging({ move, days: daysOf(distance.x) });
    },
    onEnd: (move, distance) => {
      const days = daysOf(distance.x);
      setDragging(null);
      if (days === 0) {
        onPress();
      } else {
        onEnd(move, days);
      }
    },
    onCancel: () => {
      setDragging(null);
    },
  });
  return { dragging, handlers };
}
