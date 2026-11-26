import { useRef, useState, type PointerEvent } from "react";

/** Where a drag of the background started: the pointer, and how far the box was scrolled. */
interface DragStart {
  readonly x: number;
  readonly y: number;
  readonly left: number;
  readonly top: number;
}

/** The pointer events of the scroll box of the picture. */
export interface BackgroundHandlers {
  readonly onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerUp: () => void;
  readonly onPointerCancel: () => void;
}

/** The button of a pointer that drags: the main one. */
const mainButton = 0;

/**
 * Moves the picture with its background, as a held mouse button drags it (UX §4.15). A finger
 * scrolls the box itself, and a card is pressed, never dragged.
 */
export function useBackgroundDrag(): {
  readonly dragging: boolean;
  readonly handlers: BackgroundHandlers;
} {
  const from = useRef<DragStart | null>(null);
  const [dragging, setDragging] = useState(false);
  const end = (): void => {
    from.current = null;
    setDragging(false);
  };
  const handlers: BackgroundHandlers = {
    onPointerDown: (event) => {
      const box = event.currentTarget;
      // A card is a button: it is pressed, not dragged.
      const onCard = event.target instanceof Element && event.target.closest("button") !== null;
      if (event.button !== mainButton || event.pointerType === "touch" || onCard) {
        return;
      }
      from.current = {
        x: event.clientX,
        y: event.clientY,
        left: box.scrollLeft,
        top: box.scrollTop,
      };
      box.setPointerCapture(event.pointerId);
      setDragging(true);
    },
    onPointerMove: (event) => {
      const start = from.current;
      if (start === null) {
        return;
      }
      event.currentTarget.scrollTo(
        start.left - (event.clientX - start.x),
        start.top - (event.clientY - start.y),
      );
    },
    onPointerUp: end,
    onPointerCancel: end,
  };
  return { dragging, handlers };
}
