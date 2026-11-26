import { useRef, type PointerEvent } from "react";

/** The pointer events of a part that can be dragged (a bar, a handle, the title or the corner of a box). */
export interface DragHandlers {
  readonly onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerCancel: (event: PointerEvent<HTMLElement>) => void;
}

/** How far the pointer is from where the drag started, in pixels of the page (a page scrolled meanwhile counts). */
export interface DragDistance {
  readonly x: number;
  readonly y: number;
}

/** The button of a pointer that drags: the main one (the left button of a mouse, a finger, a pen). */
const mainButton = 0;

export interface PointerDragInput<Key> {
  /** Whether the part of `key` follows the pointer once it is pressed; a part that does not is only pressed. */
  readonly drags: (key: Key) => boolean;
  /** The pointer was pressed on the part of `key` and drags it: before the first move. */
  readonly onStart?: (key: Key) => void;
  readonly onMove: (key: Key, distance: DragDistance, event: PointerEvent<HTMLElement>) => void;
  /** The pointer was let go: how far it went (nothing for a press without a move). */
  readonly onEnd: (key: Key, distance: DragDistance) => void;
  readonly onCancel: () => void;
}

/**
 * The one way the screens follow a pointer that drags a part (the bars of the timeline, the boxes of a dashboard): only
 * the main button starts it, the part keeps the pointer while it is dragged (pointer capture), and what the part does is
 * told the distance from where it started. Every drag has a typed way too (`shared/drag-kinds.ts`).
 */
export function usePointerDrag<Key>(input: PointerDragInput<Key>): {
  readonly handlers: (key: Key) => DragHandlers;
  /** Ends the drag under way without letting it go anywhere (Escape). */
  readonly cancel: () => void;
} {
  const from = useRef<{ readonly x: number; readonly y: number; readonly key: Key } | null>(null);
  const distance = (event: PointerEvent<HTMLElement>): DragDistance => ({
    x: event.pageX - (from.current?.x ?? event.pageX),
    y: event.pageY - (from.current?.y ?? event.pageY),
  });
  const cancel = (): void => {
    if (from.current !== null) {
      from.current = null;
      input.onCancel();
    }
  };
  const handlers = (key: Key): DragHandlers => ({
    onPointerDown: (event) => {
      if (event.button !== mainButton) {
        return;
      }
      // A part inside another (a handle in a bar) starts no drag of the one around it.
      event.stopPropagation();
      from.current = { x: event.pageX, y: event.pageY, key };
      if (input.drags(key)) {
        event.currentTarget.setPointerCapture(event.pointerId);
        input.onStart?.(key);
      }
    },
    onPointerMove: (event) => {
      if (from.current?.key === key && input.drags(key)) {
        input.onMove(key, distance(event), event);
      }
    },
    onPointerUp: (event) => {
      if (from.current?.key !== key) {
        return;
      }
      event.stopPropagation();
      const went = input.drags(key) ? distance(event) : { x: 0, y: 0 };
      from.current = null;
      input.onEnd(key, went);
    },
    onPointerCancel: cancel,
  });
  return { handlers, cancel };
}
