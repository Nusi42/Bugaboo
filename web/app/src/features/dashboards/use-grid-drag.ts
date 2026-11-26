import type { Packer } from "@bugaboo/wasm";
import { useEffect, useRef, useState } from "react";

import { dragKinds } from "../../shared/drag-kinds.ts";
import { usePointerDrag, type DragDistance, type DragHandlers } from "../../shared/pointer-drag.ts";
import { droppedAt, resizedTo, samePlaces, type GridPlace } from "./grid-layout.ts";
import {
  measureOf,
  rowsCrossed,
  stepsFrom,
  type GridSteps,
} from "../../shared/dashboard-grid-steps.ts";

export type { DragHandlers } from "../../shared/pointer-drag.ts";

/** What a pointer does to a box: moves it by its title, or resizes it by its corner. */
export type Gesture = typeof dragKinds.dashboardBox.kind | typeof dragKinds.dashboardCorner.kind;

/** A box that is being placed: which, how, and the places the boxes will have when it is let go. */
export interface Placing {
  readonly at: number;
  readonly gesture: Gesture;
  readonly places: readonly GridPlace[];
}

/** Which part of which box a pointer drags. */
interface Part {
  readonly at: number;
  readonly gesture: Gesture;
}

/** The places of the boxes if the box of a drag were let go this far from where it started. */
function placesAfter(
  part: Part,
  places: readonly GridPlace[],
  pack: Packer,
  { steps, distance }: { readonly steps: GridSteps; readonly distance: DragDistance },
): readonly GridPlace[] {
  const place = places[part.at];
  if (place === undefined) {
    return places;
  }
  const columns = Math.round(distance.x / steps.column);
  // A move starts from the top edge of the box, a resize from its bottom edge.
  const edge = part.gesture === dragKinds.dashboardBox.kind ? place.row : place.row + place.height;
  const rows = rowsCrossed(steps, edge, distance.y);
  return part.gesture === dragKinds.dashboardBox.kind
    ? droppedAt(places, part.at, { column: place.column + columns, row: place.row + rows }, pack)
    : resizedTo(
        places,
        part.at,
        { width: place.width + columns, height: place.height + rows },
        pack,
      );
}

/** While `on`, Escape (with no modifier) calls `cancel`. */
function useEscapeWhile(on: boolean, cancel: () => void): void {
  useEffect(() => {
    if (!on) {
      return () => {};
    }
    const listen = (event: KeyboardEvent): void => {
      if (event.key === "Escape" && !event.altKey && !event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        cancel();
      }
    };
    window.addEventListener("keydown", listen);
    return () => {
      window.removeEventListener("keydown", listen);
    };
  }, [on, cancel]);
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
 * Placing a box with a pointer (UX §4.10, `shared/pointer-drag.ts`): while it is dragged by its title or its corner,
 * the boxes stay where they are and only the shadow shows where it will land (nothing moves under the cursor, UX §6.4);
 * when it is let go, the others make room once. Escape lets go without placing it, and the pointer near the top or bottom of
 * what is shown scrolls the page. Every drag has its keys too (*Move …*, *Resize …*).
 */
export function useGridDrag({ places, pack, gridOf, onDrop }: GridDragInput): {
  readonly placing: Placing | null;
  readonly handlers: (at: number, gesture: Gesture) => DragHandlers;
} {
  const [placing, setPlacing] = useState<Placing | null>(null);
  const steps = useRef<GridSteps>({ column: 1, row: 1, rowGap: 0, rows: [] });
  const parts = useRef(new Map<string, Part>());
  const partOf = (at: number, gesture: Gesture): Part => {
    const name = `${gesture} ${String(at)}`;
    const known = parts.current.get(name);
    if (known !== undefined) {
      return known;
    }
    const made = { at, gesture };
    parts.current.set(name, made);
    return made;
  };
  const after = (part: Part, distance: DragDistance): readonly GridPlace[] =>
    pack === null ? places : placesAfter(part, places, pack, { steps: steps.current, distance });
  const drag = usePointerDrag<Part>({
    drags: () => pack !== null && gridOf() !== null,
    onStart: (part) => {
      const grid = gridOf();
      steps.current = grid === null ? steps.current : stepsFrom(measureOf(grid));
      setPlacing({ ...part, places });
    },
    scrollsAtTheEdge: true,
    onMove: (part, distance) => {
      setPlacing({ ...part, places: after(part, distance) });
    },
    onEnd: (part, distance) => {
      const placed = after(part, distance);
      setPlacing(null);
      if (!samePlaces(placed, places)) {
        onDrop({ ...part, places: placed });
      }
    },
    onCancel: () => {
      setPlacing(null);
    },
  });
  useEscapeWhile(placing !== null, drag.cancel);
  return { placing, handlers: (at, gesture) => drag.handlers(partOf(at, gesture)) };
}
