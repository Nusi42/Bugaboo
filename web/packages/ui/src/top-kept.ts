import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";

import { doNothing } from "./do-nothing.ts";
import { useLayout } from "./layout.ts";
import { LEAST_MOVE_PX } from "./least-move.ts";
import { scrollBoxOf } from "./scroll-box.ts";

/** Where the top of a part stood on the screen, in which shape, and whether it stood above the top edge of its box. */
interface Noted {
  readonly shape: string;
  /** The layout of the window then: a change of layout moves everything, and is no change of the part's shape. */
  readonly layout: string;
  readonly top: number;
  readonly above: boolean;
}

/** The box the part scrolls in. */
function boxOf(part: HTMLElement): HTMLElement | null {
  return scrollBoxOf(part.parentElement);
}

/**
 * Keeps the top of a part where it stands on the screen when `shape` changes, while that top lies above the top edge
 * of its scroll box: the browser's scroll anchoring would then keep a part under it still and move the changed part
 * (a text whose editor closes onto the saved text) instead (UX §6.4). Where the part stands is noted again after each
 * scroll of its box and each change of size of what the box holds, so a scroll by the person, or a row that came in
 * above, is not made up for.
 */
export function useTopKept(ref: Readonly<RefObject<HTMLElement | null>>, shape: string): void {
  const noted = useRef<Noted | null>(null);
  const layout = useLayout();
  useLayoutEffect(() => {
    const part = ref.current;
    const was = noted.current;
    const box = part === null ? null : boxOf(part);
    if (
      part === null ||
      box === null ||
      was === null ||
      was.shape === shape ||
      was.layout !== layout ||
      !was.above
    ) {
      return;
    }
    const moved = part.getBoundingClientRect().top - was.top;
    if (Math.abs(moved) > LEAST_MOVE_PX) {
      box.scrollTop += moved;
    }
  }, [ref, shape, layout]);
  useEffect(() => {
    const part = ref.current;
    const box = part === null ? null : boxOf(part);
    if (part === null || box === null) {
      noted.current = null;
      return doNothing;
    }
    const note = (): void => {
      const top = part.getBoundingClientRect().top;
      noted.current = { shape, layout, top, above: top < box.getBoundingClientRect().top };
    };
    note();
    const sizes = new ResizeObserver(note);
    for (const held of box.children) {
      sizes.observe(held);
    }
    box.addEventListener("scroll", note, { passive: true });
    return () => {
      sizes.disconnect();
      box.removeEventListener("scroll", note);
    };
  }, [ref, shape, layout]);
}
