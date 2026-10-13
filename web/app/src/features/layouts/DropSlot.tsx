import { partOf, part } from "@bugaboo/ui";

import { useRef } from "react";

import { useDrop } from "react-aria-components";

import { dropOptions } from "./dnd.ts";

export interface DropSlotProps {
  /** Dropping is possible: not on a phone, where the buttons are the way. */
  readonly enabled: boolean;
  /** A field was dropped here: its key. */
  readonly onDrop: (field: string) => void;
}

/** A place before a field of a section (or at its end) where a dragged field can be dropped. */
export function DropSlot({ enabled, onDrop }: DropSlotProps) {
  const ref = useRef<HTMLDivElement>(null);
  const drop = useDrop({ ref, ...dropOptions(enabled, onDrop) });
  return (
    <div
      ref={ref}
      {...drop.dropProps}
      {...partOf(part.layoutSlot, { states: [drop.isDropTarget && "over"] })}
      aria-hidden
    />
  );
}
