import { part } from "@bugaboo/ui";
import { useLayoutEffect, useRef, useState, type RefObject } from "react";

import { useShortcuts } from "../../shared/use-shortcuts.ts";

/** The rows of the grid, group headers included: what the arrow keys move over. */
const rowsOfGrid = [part.listRow, part.listGroup]
  .map((name) => `[data-rac][data-part="${name}"]`)
  .join(", ");

/**
 * `j` and `k` move the focus to the next or the previous row of a list, as the arrow keys do (UX
 * §6.2). The ref goes on the list.
 */
export function useRowKeys(): RefObject<HTMLDivElement | null> {
  const list = useRef<HTMLDivElement>(null);
  const step = (direction: 1 | -1): void => {
    const rows = list.current?.querySelectorAll<HTMLElement>(rowsOfGrid) ?? [];
    let at = -1;
    for (const [index, row] of rows.entries()) {
      at = row.contains(document.activeElement) ? index : at;
    }
    rows[Math.min(Math.max(at + direction, 0), rows.length - 1)]?.focus();
  };
  useShortcuts({
    "list-next": () => {
      step(1);
    },
    "list-previous": () => {
      step(-1);
    },
  });
  return list;
}

/** Calls `onRoom` with the width of the box now and whenever it changes; gives back what stops it. */
function watchRoom(box: HTMLElement, onRoom: (width: number) => void): () => void {
  onRoom(box.getBoundingClientRect().width);
  const watching = new ResizeObserver(() => {
    onRoom(box.getBoundingClientRect().width);
  });
  watching.observe(box);
  return () => {
    watching.disconnect();
  };
}

/**
 * Tells the room the columns have, now and whenever it changes, so the columns that do not fit can
 * give way: the width of the list without the bar its rows scroll with (a computer with a mouse
 * draws one; its room is not the columns'). Gives back what to tell the width of that bar.
 */
export function useRoom(
  getBox: () => HTMLElement | null,
  onRoom: (width: number) => void,
): (gutter: number) => void {
  const [width, setWidth] = useState<number>();
  const [gutter, setGutter] = useState(0);
  useLayoutEffect(() => {
    const box = getBox();
    return box === null ? undefined : watchRoom(box, setWidth);
  }, [getBox]);
  useLayoutEffect(() => {
    if (width !== undefined) {
      onRoom(width - gutter);
    }
  }, [width, gutter, onRoom]);
  return setGutter;
}
