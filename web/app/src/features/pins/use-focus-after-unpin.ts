import { useEffect, useRef } from "react";

import type { Pin } from "./pins.ts";

/** Where the focus goes when a pin is taken away: the menu of the next pin, or else what stands above the section. */
interface FocusAfter {
  readonly gone: string;
  readonly next: string | null;
  readonly fallback: HTMLElement | null;
}

/**
 * After *Unpin* the focus does not fall to the page: it goes to the menu button of the next pin (the one before when it
 * was the last), or to the first control above the section when no pin is left. `list` is the element the section
 * stands in, `unpinning` remembers where the focus goes, and `keepMenu` keeps the box of the menu of a pin for it to find.
 */
export function useFocusAfterUnpin(items: readonly Pin[] | undefined) {
  const list = useRef<HTMLUListElement | null>(null);
  const menus = useRef(new Map<string, HTMLElement>());
  const after = useRef<FocusAfter | null>(null);
  useEffect(() => {
    const wanted = after.current;
    if (wanted === null || items?.some((pin) => pin.id === wanted.gone) === true) {
      return;
    }
    after.current = null;
    const next = wanted.next === null ? undefined : menus.current.get(wanted.next);
    (next?.querySelector("button") ?? wanted.fallback)?.focus();
  }, [items]);
  const unpinning = (pins: readonly Pin[], at: number): void => {
    const gone = pins[at];
    const near = pins[at + 1] ?? pins[at - 1];
    if (gone === undefined) {
      return;
    }
    after.current = {
      gone: gone.id,
      next: near?.id ?? null,
      fallback:
        list.current?.closest("nav, [role=dialog]")?.querySelector<HTMLElement>("a, button") ??
        null,
    };
  };
  const keepMenu = (id: string) => (element: HTMLElement | null) => {
    if (element === null) {
      menus.current.delete(id);
    } else {
      menus.current.set(id, element);
    }
  };
  return { list, unpinning, keepMenu };
}
