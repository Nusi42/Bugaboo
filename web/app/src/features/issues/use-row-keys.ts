import { useRef, type RefObject } from "react";

import { useShortcuts } from "../../shared/use-shortcuts.ts";

/**
 * `j` and `k` move the focus to the title of the next or the previous row of a list (UX §6.2). The
 * ref goes on the list; the titles are the links inside it.
 */
export function useRowKeys(): RefObject<HTMLUListElement | null> {
  const list = useRef<HTMLUListElement>(null);
  const step = (direction: 1 | -1): void => {
    const titles = list.current?.querySelectorAll<HTMLElement>("a") ?? [];
    let at = -1;
    for (const [index, title] of titles.entries()) {
      at = title === document.activeElement ? index : at;
    }
    titles[Math.min(Math.max(at + direction, 0), titles.length - 1)]?.focus();
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
