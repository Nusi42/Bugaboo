import { useRef, useState, type PointerEvent } from "react";

import type { Move } from "./moves.ts";

/** A drag that is under way: what it moves and by how many days so far. */
export interface Dragging {
  readonly move: Move;
  readonly days: number;
}

/** The pointer events of one part of a bar that can be dragged (the bar, or one of its handles). */
export interface DragHandlers {
  readonly onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerCancel: (event: PointerEvent<HTMLElement>) => void;
}

/** The button of a pointer that drags: the main one (the left button of a mouse, a finger, a pen). */
const mainButton = 0;

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
 * changed starts no drag. Every drag has a key too (`bar-keys.ts`).
 */
export function useBarDrag({ dayPixels, mayDrag, onEnd, onPress }: BarDragInput): {
  readonly dragging: Dragging | null;
  readonly handlers: (move: Move) => DragHandlers;
} {
  const [dragging, setDragging] = useState<Dragging | null>(null);
  const from = useRef<{ readonly x: number; readonly move: Move } | null>(null);
  const daysAt = (x: number): number =>
    from.current === null || dayPixels <= 0 ? 0 : Math.round((x - from.current.x) / dayPixels);
  const handlers = (move: Move): DragHandlers => ({
    onPointerDown: (event) => {
      if (event.button !== mainButton) {
        return;
      }
      // A handle is inside the bar: its drag is not also a drag of the bar.
      event.stopPropagation();
      from.current = { x: event.clientX, move };
      if (mayDrag) {
        event.currentTarget.setPointerCapture(event.pointerId);
        setDragging({ move, days: 0 });
      }
    },
    onPointerMove: (event) => {
      if (mayDrag && from.current?.move === move) {
        setDragging({ move, days: daysAt(event.clientX) });
      }
    },
    onPointerUp: (event) => {
      if (from.current?.move !== move) {
        return;
      }
      event.stopPropagation();
      const days = mayDrag ? daysAt(event.clientX) : 0;
      from.current = null;
      setDragging(null);
      if (days === 0) {
        onPress();
      } else {
        onEnd(move, days);
      }
    },
    onPointerCancel: () => {
      from.current = null;
      setDragging(null);
    },
  });
  return { dragging, handlers };
}
