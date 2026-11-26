import { gridColumns } from "@bugaboo/api-client";
import type { Packer } from "@bugaboo/wasm";
import { useRef, useState, type PointerEvent } from "react";

import { dragKinds } from "../../shared/drag-kinds.ts";
import { droppedAt, resizedTo, samePlaces, type GridPlace } from "./grid-layout.ts";

/** What a pointer does to a box: moves it by its title, or resizes it by its corner. */
export type Gesture = typeof dragKinds.dashboardBox.kind | typeof dragKinds.dashboardCorner.kind;

/** A box that is being placed: which, how, and the places the boxes will have when it is let go. */
export interface Placing {
  readonly at: number;
  readonly gesture: Gesture;
  readonly places: readonly GridPlace[];
}

/** The pointer events of the title or the corner of a box. */
export interface DragHandlers {
  readonly onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerCancel: () => void;
}

/** The button of a pointer that drags: the main one (the left button of a mouse, a finger, a pen). */
const mainButton = 0;

/** Where a drag started, and how far one column and one row are on screen. */
interface Start {
  readonly x: number;
  readonly y: number;
  readonly at: number;
  readonly gesture: Gesture;
  readonly column: number;
  readonly row: number;
}

/** A length the browser computed, in pixels (`72px`); none it cannot read. */
function pixels(length: string): number {
  return Number(length.replace("px", "")) || 0;
}

/** How far a column and a row of the grid are on screen: a column of its width and a row of its height, each with the gap. */
function stepsOf(grid: HTMLElement): { column: number; row: number } {
  const style = getComputedStyle(grid);
  const columnGap = pixels(style.columnGap);
  return {
    column: (grid.clientWidth + columnGap) / gridColumns,
    row: pixels(style.gridAutoRows) + pixels(style.rowGap),
  };
}

/** The places of the boxes if the box of a drag were let go where the pointer is now. */
function placesAfter(
  from: Start,
  places: readonly GridPlace[],
  pack: Packer,
  pointer: { readonly x: number; readonly y: number },
): readonly GridPlace[] {
  const place = places[from.at];
  if (place === undefined) {
    return places;
  }
  const columns = Math.round((pointer.x - from.x) / from.column);
  const rows = Math.round((pointer.y - from.y) / from.row);
  return from.gesture === dragKinds.dashboardBox.kind
    ? droppedAt(places, from.at, { column: place.column + columns, row: place.row + rows }, pack)
    : resizedTo(
        places,
        from.at,
        { width: place.width + columns, height: place.height + rows },
        pack,
      );
}

export interface GridDragInput {
  readonly places: readonly GridPlace[];
  /** The packing of the server; until it is loaded, nothing is dragged. */
  readonly pack: Packer | null;
  /** The element of the grid, which its steps are measured on. */
  readonly gridOf: () => HTMLElement | null;
  /** A box was let go somewhere else: the places of every box after it. */
  readonly onDrop: (placing: Placing) => void;
}

/**
 * Placing a box with a pointer (UX §4.10): while it is dragged by its title or its corner, the boxes stay where they
 * are and only the shadow shows where it will land (nothing moves under the cursor, UX §6.4); when it is let go, the
 * others make room once. Every drag has its keys too (*Move …*, *Resize …*).
 */
export function useGridDrag({ places, pack, gridOf, onDrop }: GridDragInput): {
  readonly placing: Placing | null;
  readonly handlers: (at: number, gesture: Gesture) => DragHandlers;
} {
  const [placing, setPlacing] = useState<Placing | null>(null);
  const start = useRef<Start | null>(null);
  const placesAt = (x: number, y: number): readonly GridPlace[] =>
    start.current === null || pack === null
      ? places
      : placesAfter(start.current, places, pack, { x, y });
  const end = (): void => {
    start.current = null;
    setPlacing(null);
  };
  const handlers = (at: number, gesture: Gesture): DragHandlers => ({
    onPointerDown: (event) => {
      const grid = gridOf();
      if (event.button !== mainButton || pack === null || grid === null) {
        return;
      }
      event.preventDefault();
      const steps = stepsOf(grid);
      start.current = { x: event.clientX, y: event.clientY, at, gesture, ...steps };
      event.currentTarget.setPointerCapture(event.pointerId);
      setPlacing({ at, gesture, places });
    },
    onPointerMove: (event) => {
      if (start.current?.at === at && start.current.gesture === gesture) {
        setPlacing({ at, gesture, places: placesAt(event.clientX, event.clientY) });
      }
    },
    onPointerUp: (event) => {
      if (start.current?.at !== at || start.current.gesture !== gesture) {
        return;
      }
      const after = placesAt(event.clientX, event.clientY);
      end();
      if (!samePlaces(after, places)) {
        onDrop({ at, gesture, places: after });
      }
    },
    onPointerCancel: end,
  });
  return { placing, handlers };
}
