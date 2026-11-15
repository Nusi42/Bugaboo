import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";

import { doNothing } from "./do-nothing.ts";
import { scrollBoxOf } from "./scroll-box.ts";

/** A difference of more than half a pixel is a move (UX §6.4); less is the browser's subpixel layout. */
const LEAST_MOVE = 0.5;

/** Where the top of a part stood on the screen, and in which shape. */
interface Noted {
  readonly shape: string;
  readonly top: number;
}

/**
 * Keeps the top of a part where it stands on the screen when `shape` changes: the part a person changed (a text whose
 * editor closes onto the saved text) stays put, and only what stands under it moves up or down with it (UX §6.4). The
 * browser's scroll anchoring would otherwise keep a part under it still and move the changed part instead, when the
 * part's top lies above the box's top edge. Where the part stands is noted once it has its shape and each time its box
 * scrolls, so a scroll by the person is theirs.
 */
export function useTopKept(ref: Readonly<RefObject<HTMLElement | null>>, shape: string): void {
  const noted = useRef<Noted | null>(null);
  useLayoutEffect(() => {
    const part = ref.current;
    const was = noted.current;
    if (part === null || was === null || was.shape === shape) {
      return;
    }
    const box = scrollBoxOf(part.parentElement);
    const moved = part.getBoundingClientRect().top - was.top;
    if (box !== null && Math.abs(moved) > LEAST_MOVE) {
      box.scrollTop += moved;
    }
  }, [ref, shape]);
  useEffect(() => {
    const part = ref.current;
    const box = part === null ? null : scrollBoxOf(part.parentElement);
    if (part === null || box === null) {
      noted.current = null;
      return doNothing;
    }
    const note = (): void => {
      noted.current = { shape, top: part.getBoundingClientRect().top };
    };
    note();
    box.addEventListener("scroll", note, { passive: true });
    return () => {
      box.removeEventListener("scroll", note);
    };
  }, [ref, shape]);
}
