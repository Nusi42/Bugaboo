import { Icon, iconName, partOf, part, type PartAttributes } from "@bugaboo/ui";
import { useRef, type ReactNode } from "react";
import { Button as AriaButton, useDrag } from "react-aria-components";

import { typeOf, useDropOf } from "./drop-of.ts";

/**
 * Dragging things of a list to another place in the list (or in another list), with a mouse or a
 * finger. Every drag is also possible with buttons that the screen offers next to it (*Move up*,
 * *Move to …*): this is the quick way, never the only one, and the way for the keyboard and for
 * screen readers. The handle and the places to drop are therefore hidden from assistive technology
 * together: a drag with a screen reader could be started at the handle and not finished at a
 * place. What is dragged is named by a `kind` (a word of the screen: `layout-field`), so
 * only a thing of that kind can be dropped on a place made for it.
 */

export interface DraggableHeadProps {
  /** What is dragged: a word of the screen, like `layout-field`. */
  readonly kind: string;
  /** Which one: the key of the thing. */
  readonly id: string;
  /** The name of the handle, like *Drag Due date*: not announced, the handle is for the pointer. */
  readonly label: string;
  /** Whether it can be dragged at all: on a phone there is no handle. */
  readonly canDrag: boolean;
  /** The words and buttons next to the handle. */
  readonly children: ReactNode;
}

/**
 * The top line of a thing that can be dragged: a mouse or a finger drags it from here, and the
 * buttons next to it do the same things without a drag (the way for the keyboard and for screen
 * readers; the handle is hidden from them and left out of the tab order).
 */
export function DraggableHead({ kind, id, label, canDrag, children }: DraggableHeadProps) {
  const drag = useDrag({
    getItems: () => [{ [typeOf(kind)]: id }],
    getAllowedDropOperations: () => ["move"],
    hasDragButton: true,
    isDisabled: !canDrag,
  });
  return (
    <div
      {...partOf(part.draggable, { states: [drag.isDragging && "dragging"] })}
      {...drag.dragProps}
    >
      {canDrag ? (
        // The button drops `aria-hidden`, so the wrapper hides it from assistive technology.
        <span aria-hidden>
          <AriaButton
            {...drag.dragButtonProps}
            {...partOf(part.draggableHandle)}
            aria-label={label}
            excludeFromTabOrder
          >
            <Icon name={iconName.menu} />
          </AriaButton>
        </span>
      ) : null}
      {children}
    </div>
  );
}

export interface DropSlotProps {
  /** What may be dropped here: the same word as the `kind` of what is dragged. */
  readonly kind: string;
  /** Dropping is possible: not on a phone, where the buttons are the way. */
  readonly enabled: boolean;
  /** Something was dropped here: its `id`. */
  readonly onDrop: (id: string) => void;
}

/** A place before a thing of a list (or at its end) where a dragged thing of the kind can be dropped. */
export function DropSlot({ kind, enabled, onDrop }: DropSlotProps) {
  const ref = useRef<HTMLDivElement>(null);
  const drop = useDropOf(kind, ref, enabled, (id) => {
    onDrop(id);
  });
  return (
    <div
      ref={ref}
      {...drop.dropProps}
      {...partOf(part.dropSlot, { states: [drop.isDropTarget && "over"] })}
      aria-hidden
    />
  );
}

export interface DragHandleProps {
  /** What is dragged: a word of the screen. */
  readonly kind: string;
  /** Which one: the key of the thing. */
  readonly id: string;
  /** The name of the handle: not announced, the handle is for the pointer. */
  readonly label: string;
  /** How the handle looks: its part. */
  readonly look: PartAttributes;
  /** What the handle shows, if anything (the dot of a bar shows nothing). */
  readonly children?: ReactNode;
}

/**
 * A handle alone that a pointer drags (the dot at the end of a bar of the timeline): hidden from
 * assistive technology and out of the tab order, because the screen offers the same with a button
 * next to it (*Add dependency…*).
 */
export function DragHandle({ kind, id, label, look, children }: DragHandleProps) {
  const drag = useDrag({
    getItems: () => [{ [typeOf(kind)]: id }],
    getAllowedDropOperations: () => ["move"],
    hasDragButton: true,
  });
  return (
    <span aria-hidden {...drag.dragProps}>
      <AriaButton {...drag.dragButtonProps} {...look} aria-label={label} excludeFromTabOrder>
        {children}
      </AriaButton>
    </span>
  );
}
