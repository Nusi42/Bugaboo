import { useRef, type RefObject } from "react";

import { useShortcuts } from "../../shared/use-shortcuts.ts";

/** The rows of the grid, group headers included: what the arrow keys move over. */
const rowsOfGrid = '[data-rac][data-part="list-row"], [data-rac][data-part="list-group"]';

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
