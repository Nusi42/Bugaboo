import { t } from "@bugaboo/messages";
import type { KeyboardEvent } from "react";

import { dragKinds } from "../../shared/drag-kinds.ts";
import type { BoxActions } from "./BoxMenu.tsx";
import { bodyOf, type Box, type Dashboard } from "./dashboards.ts";
import { swappedInOrder } from "./grid-layout.ts";
import { useBoxChanges } from "./use-box-changes.ts";
import { useBoxKeys } from "./use-box-keys.ts";
import { useGridDrag, type DragHandlers, type Gesture, type Placing } from "./use-grid-drag.ts";
import { usePacker } from "./use-packer.ts";

/** How a phone places a box: *Move up* and *Move down*. */
function phoneWays(at: number, swap: (at: number, up: boolean) => void) {
  return {
    onUp: () => {
      swap(at, true);
    },
    onDown: () => {
      swap(at, false);
    },
  };
}

/** How a computer places a box with the keys: *Move …* and *Resize …*. */
function computerWays(at: number, start: (at: number, gesture: Gesture) => void) {
  return {
    onMove: () => {
      start(at, dragKinds.dashboardBox.kind);
    },
    onResize: () => {
      start(at, dragKinds.dashboardCorner.kind);
    },
  };
}

/** What a box of the grid is told about placing: the pointer events, which box is placed, its menu. */
export interface GridControls {
  /** A pointer can drag the boxes: a computer in *Change*, once the packing is loaded. */
  readonly draggable: boolean;
  readonly handlers: (at: number, gesture: Gesture) => DragHandlers;
  /** The box being dragged, if one is. */
  readonly dragged: number | undefined;
  /** The box the keys of *Move …* or *Resize …* work on, if one is. */
  readonly keyed: number | undefined;
  readonly actionsOf: (box: Box, at: number, onChangeBox: (box: Box) => void) => BoxActions;
  /** The keys of the handle of a box while *Move …* or *Resize …* works on it. */
  readonly onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
  /** The focus left the handle: placing ends unless it stays in the box. */
  readonly onBlur: (stillInBox: boolean) => void;
  /** The handle was pressed (Enter, Space): *Move …* of the box starts. */
  readonly onPressHandle: (at: number) => void;
}

/** What the boxes are told about placing, from the drag of a pointer and the keys. */
function controlsOf({
  draggable,
  drag,
  keys,
  actionsOf,
}: {
  readonly draggable: boolean;
  readonly drag: ReturnType<typeof useGridDrag>;
  readonly keys: ReturnType<typeof useBoxKeys>;
  readonly actionsOf: GridControls["actionsOf"];
}): GridControls {
  return {
    draggable,
    handlers: drag.handlers,
    dragged: drag.placing?.at,
    keyed: keys.placing?.at,
    actionsOf,
    onKeyDown: keys.onKeyDown,
    onBlur: keys.onBlur,
    onPressHandle: (at) => {
      if (keys.placing === null) {
        keys.start(at, dragKinds.dashboardBox.kind);
      }
    },
  };
}

/**
 * How the boxes of a dashboard are placed in *Change*: by a pointer, by the keys of their menus, or on a phone by *Move
 * up* and *Move down*; each placing is saved with the places the server's packing gives, and offers *Undo*.
 */
export function useGridPlacing(
  dashboard: Dashboard,
  gridOf: () => HTMLElement | null,
  { changing, phone }: { readonly changing: boolean; readonly phone: boolean },
) {
  const pack = usePacker(changing);
  const { boxes } = dashboard;
  const changes = useBoxChanges(dashboard);
  const placed = (placing: Placing): void => {
    const title = boxes[placing.at]?.title ?? "";
    const said =
      placing.gesture === dragKinds.dashboardBox.kind ? "dashboards.moved" : "dashboards.resized";
    changes.place(placing.places, t(said, { title }));
  };
  const drag = useGridDrag({ places: boxes, pack, gridOf, onDrop: placed });
  const keys = useBoxKeys({ places: boxes, pack, changing, onDrop: placed });
  const swap = (at: number, up: boolean): void => {
    const places = pack === null ? undefined : swappedInOrder(boxes, at, up, pack);
    if (places !== undefined) {
      placed({ at, gesture: dragKinds.dashboardBox.kind, places });
    }
  };
  const actionsOf = (box: Box, at: number, onChangeBox: (box: Box) => void): BoxActions => ({
    // Until the packing is loaded nothing can be placed, so the ways to place are not offered yet.
    ...(pack === null ? {} : phone ? phoneWays(at, swap) : computerWays(at, keys.start)),
    onChange: () => {
      onChangeBox(box);
    },
    onRemove: () => {
      changes.save({
        boxes: boxes.filter((other) => other.id !== box.id).map((other) => bodyOf(other)),
        said: t("dashboards.removed", { title: box.title }),
      });
    },
  });
  // `keyed` of the answer is where the keys of a box put it now, for the words of a screen reader.
  const controls = controlsOf({
    draggable: changing && !phone && pack !== null,
    drag,
    keys,
    actionsOf,
  });
  return {
    controls,
    refused: changes.refused,
    keyed: keys.placing,
    /** The box being placed, by a pointer or by the keys: its shadow shows where it lands. */
    placing: drag.placing ?? keys.placing,
  };
}
