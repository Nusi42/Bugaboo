import { useEffect, useRef, type RefObject } from "react";

/**
 * Gives back a ref for a container, and moves the focus to the heading inside it when `key`
 * changes (and not on the first showing). A step of a wizard replaces the one before it, so a
 * screen reader would otherwise stay on a button that is gone.
 */
export function useFocusHeading(key: string): RefObject<HTMLDivElement | null> {
  const container = useRef<HTMLDivElement>(null);
  const shown = useRef(key);
  useEffect(() => {
    if (shown.current !== key) {
      shown.current = key;
      container.current?.querySelector<HTMLElement>("h1")?.focus();
    }
  }, [key]);
  return container;
}
