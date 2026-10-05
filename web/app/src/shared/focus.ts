import { useLocation } from "@tanstack/react-router";
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

/**
 * When the person goes to another page, the focus moves to the heading of the new page (or to the
 * content), so a screen reader reads the new page instead of staying on the link that was pressed.
 * Not on the first showing, and not when only the search of the address changes.
 */
export function usePageFocus(contentId: string): void {
  const { pathname } = useLocation();
  const shownPage = useRef(pathname);
  useEffect(() => {
    const changed = shownPage.current !== pathname;
    shownPage.current = pathname;
    // After the next paint: a layer that closes (quick search) gives the focus back to where it
    // was while it closes, and the heading must not lose it to that.
    const frame = changed
      ? requestAnimationFrame(() => {
          const content = document.querySelector<HTMLElement>(`#${contentId}`);
          (content?.querySelector<HTMLElement>("h1") ?? content)?.focus();
        })
      : 0;
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [pathname, contentId]);
}
