import { Icon, iconName, partOf, part } from "@bugaboo/ui";
import { useRef, type ReactNode } from "react";
import { Button as AriaButton, useDrag, useDrop } from "react-aria-components";

/**
 * Dragging things of a list to another place in the list (or in another list), with a mouse or a
 * finger, and the handle that starts a drag with the keyboard. Every drag is also possible with
 * buttons that the screen offers next to it (*Move up*, *Move to …*): this is the quick way, never
 * the only one. What is dragged is named by a `kind` (a word of the screen: `layout-field`), so
 * only a thing of that kind can be dropped on a place made for it.
 */

/** The type of the thing in the drag and drop of the browser: a technical word, not text for people. */
function typeOf(kind: string): string {
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

/** What a drop tells the list. */
interface Drop {
  readonly items: readonly (DroppedText | DroppedOther)[];
}

/** What a drag brings with it that a drop target can ask for. */
interface Types {
  readonly has: (type: string) => boolean;
}

export interface DraggableHeadProps {
  /** What is dragged: a word of the screen, like `layout-field`. */
  readonly kind: string;
  /** Which one: the key of the thing. */
  readonly id: string;
  /** The name of the handle for screen readers, like *Drag Due date*. */
  readonly label: string;
  /** Whether it can be dragged at all: on a phone there is no handle. */
  readonly canDrag: boolean;
  /** The words and buttons next to the handle. */
  readonly children: ReactNode;
}

/**
 * The top line of a thing that can be dragged: a mouse or a finger drags it from here, the handle
 * starts a drag with the keyboard, and the buttons next to it do the same things without a drag.
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
        <AriaButton {...drag.dragButtonProps} {...partOf(part.draggableHandle)} aria-label={label}>
          <Icon name={iconName.menu} />
        </AriaButton>
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
  const type = typeOf(kind);
  const drop = useDrop({
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
            onDrop(text);
          }
        }
      });
    },
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
