import { scrollBoxOf } from "@bugaboo/ui";
import { useRef, type PointerEvent } from "react";

import { useEdgeScroll } from "./edge-scroll.ts";

/** The pointer events of a part that can be dragged (a bar, a handle, the title or the corner of a box). */
export interface DragHandlers {
  readonly onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerCancel: (event: PointerEvent<HTMLElement>) => void;
}

/**
 * How far the pointer is from where the drag started, in pixels of the content: a scroll of the box the part stands in
 * (the main part of the layout) or of the window while it is dragged counts too.
 */
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
  readonly onMove: (key: Key, distance: DragDistance) => void;
  /** The pointer was let go: how far it went (nothing for a press without a move). */
  readonly onEnd: (key: Key, distance: DragDistance) => void;
  readonly onCancel: () => void;
  /**
   * The pointer held near the top or bottom edge of the box the part stands in scrolls that box (`shared/edge-scroll.ts`),
   * so a part can go past what is shown (a box of a dashboard below the window).
   */
  readonly scrollsAtTheEdge?: boolean;
}

/** Where a drag started: the pointer on the page, the box that scrolls and how far it was scrolled, and the part. */
interface Start<Key> {
  readonly x: number;
  readonly y: number;
  readonly box: HTMLElement | null;
  readonly top: number;
  readonly left: number;
  readonly key: Key;
}

/** Where a drag of the part of `key` starts: the pointer, and the box the part stands in with how far it is scrolled. */
function startOf<Key>(event: PointerEvent<HTMLElement>, key: Key): Start<Key> {
  const box = scrollBoxOf(event.currentTarget);
  return {
    x: event.pageX,
    y: event.pageY,
    box,
    top: box?.scrollTop ?? 0,
    left: box?.scrollLeft ?? 0,
    key,
  };
}

/** How far the pointer is from where the drag started, a scroll of its box meanwhile counted. */
function distanceFrom<Key>(
  start: Start<Key> | null,
  event: Readonly<{ pageX: number; pageY: number }>,
): DragDistance {
  if (start === null) {
    return { x: 0, y: 0 };
  }
  const { box } = start;
  return {
    x: event.pageX - start.x + (box === null ? 0 : box.scrollLeft - start.left),
    y: event.pageY - start.y + (box === null ? 0 : box.scrollTop - start.top),
  };
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
  const from = useRef<Start<Key> | null>(null);
  const edge = useEdgeScroll(input.scrollsAtTheEdge === true);
  const cancel = (): void => {
    edge.stop();
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
      from.current = startOf(event, key);
      if (input.drags(key)) {
        event.currentTarget.setPointerCapture(event.pointerId);
        input.onStart?.(key);
      }
    },
    onPointerMove: (event) => {
      if (from.current?.key === key && input.drags(key)) {
        const point = { pageX: event.pageX, pageY: event.pageY };
        edge.follow(from.current.box, event.clientY, () => {
          input.onMove(key, distanceFrom(from.current, point));
        });
        input.onMove(key, distanceFrom(from.current, event));
      }
    },
    onPointerUp: (event) => {
      if (from.current?.key !== key) {
        return;
      }
      event.stopPropagation();
      edge.stop();
      const went = input.drags(key) ? distanceFrom(from.current, event) : { x: 0, y: 0 };
      from.current = null;
      input.onEnd(key, went);
    },
    onPointerCancel: cancel,
  });
  return { handlers, cancel };
}
