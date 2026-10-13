import { t } from "@bugaboo/messages";
import { Icon, iconName, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";
import { Button as AriaButton } from "react-aria-components";

import { useFieldDrag } from "./dnd.ts";

export interface DraggableHeadProps {
  /** The key of the field. */
  readonly field: string;
  /** What the field is called, for the name of the handle. */
  readonly name: string;
  /** Whether fields can be dragged at all: on a phone there is no handle. */
  readonly canDrag: boolean;
  /** The buttons and words next to the handle. */
  readonly children: ReactNode;
}

/**
 * The top line of a field in the editor: a mouse or a finger drags the field from here, the handle
 * starts a drag with the keyboard, and the buttons next to it do the same things without a drag.
 */
export function DraggableHead({ field, name, canDrag, children }: DraggableHeadProps) {
  const drag = useFieldDrag(field, canDrag);
  return (
    <div
      {...partOf(part.layoutFieldHead, { states: [drag.isDragging && "dragging"] })}
      {...drag.dragProps}
    >
      {canDrag ? (
        <AriaButton
          {...drag.dragButtonProps}
          {...partOf(part.layoutFieldHandle)}
          aria-label={t("layouts.field-drag", { name })}
        >
          <Icon name={iconName.menu} />
        </AriaButton>
      ) : null}
      {children}
    </div>
  );
}
