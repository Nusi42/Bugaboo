import { useDrag } from "react-aria-components";

/**
 * The kind of thing a field of a layout is when it is dragged: a word of the drag and drop of the
 * browser, not text for people. Only a field of the editor can be dropped on the editor.
 */
const fieldType = "application/x-bugaboo-layout-field";

/** A dropped item that is text, which a field of the editor is. */
interface DroppedText {
  readonly kind: "text";
  readonly getText: (type: string) => Promise<string>;
}

/** An item that is not text (a file, a folder): nothing the editor takes. */
interface DroppedOther {
  readonly kind: "file" | "directory";
}

/** What a drop tells the editor. */
interface Drop {
  readonly items: readonly (DroppedText | DroppedOther)[];
}

/** What a drag brings with it that a drop target can ask for. */
interface Types {
  readonly has: (type: string) => boolean;
}

/** What a field being dragged is made of. */
export interface FieldDrag {
  /** Spread on the part that can be dragged with a mouse or a finger. */
  readonly dragProps: ReturnType<typeof useDrag>["dragProps"];
  /** For the button that starts a drag with the keyboard. */
  readonly dragButtonProps: ReturnType<typeof useDrag>["dragButtonProps"];
  readonly isDragging: boolean;
}

/** A field that can be dragged, with a button that starts the drag from the keyboard too. */
export function useFieldDrag(field: string, enabled: boolean): FieldDrag {
  return useDrag({
    getItems: () => [{ [fieldType]: field }],
    getAllowedDropOperations: () => ["move"],
    hasDragButton: true,
    isDisabled: !enabled,
  });
}

/**
 * How a place takes a dragged field, for `useDrop({ ref, ...dropOptions(enabled, onDrop) })`:
 * only a field of the editor is taken, and `onDrop` gets its key.
 */
export function dropOptions(enabled: boolean, onDrop: (field: string) => void) {
  return {
    isDisabled: !enabled,
    getDropOperation: (types: Types) =>
      types.has(fieldType) ? ("move" as const) : ("cancel" as const),
    onDrop: (event: Drop) => {
      const texts = event.items.map((item) =>
        item.kind === "text" ? item.getText(fieldType) : Promise.resolve(null),
      );
      void Promise.all(texts).then((found: readonly (string | null)[]) => {
        for (const text of found) {
          if (text !== null) {
            onDrop(text);
          }
        }
      });
    },
  };
}
