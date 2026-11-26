import { pixelsOf } from "@bugaboo/ui";
import { useEffect, useRef } from "react";

/** How often a part held at the edge of its box scrolls the box a step, in milliseconds. */
const EDGE_STEP_EVERY_MS = 100;

/** The step `box` scrolls with the pointer at `pointerY`: down near its bottom edge, up near its top edge, else none. */
function edgeStep(
  { top, bottom }: { readonly top: number; readonly bottom: number },
  pointerY: number,
): number {
  const edge = pixelsOf("size-control");
  if (pointerY > Math.min(bottom, window.innerHeight) - edge) {
    return edge;
  }
  return pointerY < Math.max(top, 0) + edge ? -edge : 0;
}

/** What a held edge scroll needs: the box, where the pointer is, and what to tell after each step. */
interface Held {
  readonly box: HTMLElement;
  readonly pointerY: number;
  readonly onScrolled: () => void;
}

/**
 * Scrolling while a part is dragged (UX §4.10): the pointer near the top or bottom edge of the box the part stands in
 * scrolls that box a step at once and then every 100 ms while it is held there, without moving (each step at once, not
 * smoothly, so the steps add up), until it leaves the edge, the box can go no further, or the drag ends. After each step
 * `onScrolled` is told, so what is dragged follows the content. One helper for every drag that scrolls (the boxes of a
 * dashboard; the calendar can use it too). While `on` is false it does nothing.
 */
export function useEdgeScroll(on: boolean): {
  readonly follow: (box: HTMLElement | null, pointerY: number, onScrolled: () => void) => void;
  readonly stop: () => void;
} {
  const held = useRef<Held | null>(null);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);
  const stop = (): void => {
    if (timer.current !== null) {
      clearInterval(timer.current);
      timer.current = null;
    }
    held.current = null;
  };
  const step = (): void => {
    const now = held.current;
    const by = now === null ? 0 : edgeStep(now.box.getBoundingClientRect(), now.pointerY);
    const before = now?.box.scrollTop;
    now?.box.scrollBy({ top: by, behavior: "instant" });
    if (now === null || by === 0 || now.box.scrollTop === before) {
      stop();
      return;
    }
    now.onScrolled();
  };
  useEffect(
    () => () => {
      if (timer.current !== null) {
        clearInterval(timer.current);
      }
    },
    [],
  );
  return {
    follow: (box, pointerY, onScrolled) => {
      if (!on || box === null) {
        return;
      }
      held.current = { box, pointerY, onScrolled };
      if (timer.current === null && edgeStep(box.getBoundingClientRect(), pointerY) !== 0) {
        step();
        timer.current = setInterval(step, EDGE_STEP_EVERY_MS);
      }
    },
    stop,
  };
}
