import { type RefObject } from "react";
import { useDrag, useDrop } from "react-aria-components";

/**
 * The drop of the screen's drag and drop (`drag-and-drop.tsx`): what is dragged is named by a `kind`
 * (a word of the screen), so only a thing of that kind can be dropped on a place made for it.
 */

/** The type of the thing in the drag and drop of the browser: a technical word, not text for people. */
export function typeOf(kind: string): string {
  return `application/x-bugaboo-${kind}`;
}

/** A dropped item that is text, which a thing of ours is. */
interface DroppedText {
  readonly kind: "text";
  readonly getText: (type: string) => Promise<string>;
}

/** An item that is not text (a file, a folder): nothing a list takes. */
interface DroppedOther {
  readonly kind: "file" | "directory";
}

/** What a drop tells the list: what was dropped, and where inside the place. */
interface Drop {
  readonly items: readonly (DroppedText | DroppedOther)[];
  readonly x: number;
  readonly y: number;
}

/** What a drag brings with it that a drop target can ask for. */
interface Types {
  readonly has: (type: string) => boolean;
}

/** Where something was dropped, from the start of the place it was dropped on. */
export interface DropPoint {
  readonly x: number;
  readonly y: number;
}

/**
 * Makes `ref` a place where a dragged thing of `kind` can be dropped: `onDrop` gets its `id` and
 * the point inside the place where it was let go. The one drop of the screen (a slot of a list, a
 * bar of the timeline, the chart itself).
 */
export function useDropOf(
  kind: string,
  ref: Readonly<RefObject<HTMLElement | null>>,
  enabled: boolean,
  onDrop: (id: string, at: DropPoint) => void,
) {
  const type = typeOf(kind);
  return useDrop({
    ref,
    isDisabled: !enabled,
    getDropOperation: (types: Types) => (types.has(type) ? "move" : "cancel"),
    onDrop: (event: Drop) => {
      const texts = event.items.map((item) =>
        item.kind === "text" ? item.getText(type) : Promise.resolve(null),
      );
      void Promise.all(texts).then((found: readonly (string | null)[]) => {
        for (const text of found) {
          if (text !== null) {
            onDrop(text, { x: event.x, y: event.y });
          }
        }
      });
    },
  });
}

/** How a thing is dragged: whether it can be, from where, and what the screen hears of it. */
export interface DragOptions {
  /** Whether it can be dragged at all: not on a phone, where the buttons are the way. */
  readonly enabled?: boolean;
  /** The thing is dragged by a handle of its own (`DraggableHead`, `DragHandle`), not as a whole. */
  readonly hasDragButton?: boolean;
  /** The drag begins. */
  readonly onStart?: () => void;
  /** The drag ends, dropped or not. */
  readonly onEnd?: () => void;
}

/**
 * Makes a thing draggable by the pointer, with `id` named as a thing of `kind`: the one drag of the
 * screens (a card of a board as a whole, a head or a handle by its grip). The counterpart of
 * `useDropOf`. A screen that drags offers the same by keys (TEST-A11Y).
 */
export function useDragOf(kind: string, id: string, options: DragOptions = {}) {
  return useDrag({
    getItems: () => [{ [typeOf(kind)]: id }],
    getAllowedDropOperations: () => ["move"],
    isDisabled: options.enabled === false,
    ...(options.hasDragButton === true ? { hasDragButton: true } : {}),
    ...(options.onStart === undefined ? {} : { onDragStart: options.onStart }),
    ...(options.onEnd === undefined ? {} : { onDragEnd: options.onEnd }),
  });
}
