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

/** What can be pressed in a row. */
const PRESSABLE = "button, a[href]";

/**
 * Where the focus goes when the row that has it goes away: the next row's button or link, else the
 * one before it, else the heading of the list. Without it the focus falls to the page and a
 * screen reader starts again from the top.
 */
export function focusTargetAfterRemoval(): HTMLElement | null {
  const row = document.activeElement?.closest("li") ?? null;
  const list = row?.parentElement ?? null;
  if (row === null || list === null) {
    return null;
  }
  const rows = [...list.children];
  const at = rows.indexOf(row);
  const around = [...rows.slice(at + 1), ...rows.slice(0, at).toReversed()];
  for (const other of around) {
    const pressable = other.querySelector<HTMLElement>(PRESSABLE);
    if (pressable !== null) {
      return pressable;
    }
  }
  return list.closest("section, main")?.querySelector<HTMLElement>("h1, h2") ?? null;
}

/** How many frames to wait for the row to go away before giving up. */
const FRAMES_TO_WAIT = 60;

/**
 * Runs a change that removes the row that has the focus (a press on its button) and moves the focus
 * as `focusTargetAfterRemoval` says, as soon as the row is gone. If the row never goes (the change
 * was refused), the focus stays where it is.
 */
export function removeKeepingFocus(run: () => void): void {
  const from = document.activeElement;
  const target = focusTargetAfterRemoval();
  run();
  if (target === null || from === null) {
    return;
  }
  const wait = (framesLeft: number): void => {
    if (!from.isConnected) {
      target.focus();
    } else if (framesLeft > 0) {
      requestAnimationFrame(() => {
        wait(framesLeft - 1);
      });
    }
  };
  requestAnimationFrame(() => {
    wait(FRAMES_TO_WAIT);
  });
}

/**
 * Where the focus goes after *Undo* brought something back: the thing that was restored (`find`
 * gives it once it is on the page again), else the heading of the content. The button that was
 * pressed is gone by then, and a screen reader would start again from the top.
 */
export function focusRestored(
  find: (() => HTMLElement | null) | undefined,
  contentId: string,
): void {
  const wait = (framesLeft: number): void => {
    const restored = find?.() ?? null;
    if (restored !== null) {
      restored.focus();
    } else if (find !== undefined && framesLeft > 0) {
      requestAnimationFrame(() => {
        wait(framesLeft - 1);
      });
    } else {
      const content = document.querySelector<HTMLElement>(`#${contentId}`);
      (content?.querySelector<HTMLElement>("h1") ?? content)?.focus();
    }
  };
  requestAnimationFrame(() => {
    wait(FRAMES_TO_WAIT);
  });
}
