import { useEffect, type RefObject } from "react";

import { doNothing } from "./do-nothing.ts";
import { scrollBoxOf } from "./scroll-box.ts";

/** A point of the pointer in the window. */
interface Point {
  readonly x: number;
  readonly y: number;
}

/** The scroll box of the pane that the head is laid over at the point: the one whose content shows under it there. */
function boxUnder(pane: HTMLElement, head: HTMLElement, point: Point): HTMLElement | null {
  for (const found of document.elementsFromPoint(point.x, point.y)) {
    if (!pane.contains(found) || head.contains(found)) {
      continue;
    }
    const box = scrollBoxOf(found instanceof HTMLElement ? found : found.parentElement, pane);
    if (box !== null) {
      return box;
    }
  }
  return null;
}

/** Where a drag is over something: all the head reads of its event. */
interface DragOver {
  readonly clientX: number;
  readonly clientY: number;
  readonly target: unknown;
}

/**
 * Scrolls the box under the pointer while a drag is held over `head`, until it leaves the head or ends; gives back what
 * stops listening.
 */
function scrollWhileHeldOver(head: HTMLElement, pane: HTMLElement): () => void {
  let held: Point | null = null;
  let frame = 0;
  const scroll = (): void => {
    if (held === null) {
      frame = 0;
      return;
    }
    const box = boxUnder(pane, head, held);
    if (box !== null) {
      box.scrollTop -= Math.max(1, head.getBoundingClientRect().bottom - held.y);
    }
    frame = requestAnimationFrame(scroll);
  };
  const stop = (): void => {
    held = null;
    cancelAnimationFrame(frame);
    frame = 0;
    document.removeEventListener("dragover", elsewhere, true);
    document.removeEventListener("drop", stop, true);
    document.removeEventListener("dragend", stop, true);
  };
  function elsewhere(event: DragOver): void {
    if (!(event.target instanceof Node) || !head.contains(event.target)) {
      stop();
    }
  }
  const onHead = (event: DragOver): void => {
    held = { x: event.clientX, y: event.clientY };
    if (frame === 0) {
      document.addEventListener("dragover", elsewhere, true);
      document.addEventListener("drop", stop, true);
      document.addEventListener("dragend", stop, true);
      frame = requestAnimationFrame(scroll);
    }
  };
  head.addEventListener("dragover", onHead);
  return () => {
    stop();
    head.removeEventListener("dragover", onHead);
  };
}

/**
 * While a dragged thing is held over the head of a pane that is laid over its boxes, the box under the pointer scrolls
 * up, as a box scrolls when a drag comes near its top edge: the head stands where that edge is, and the browser does
 * not scroll a box for a pointer over another part (UX §6.5). The deeper the pointer is in the head, the faster the box
 * scrolls; the drag leaving the head, a drop or the end of the drag stops it.
 */
export function useScrollUnderHead(
  head: Readonly<RefObject<HTMLElement | null>>,
  pane: Readonly<RefObject<HTMLElement | null>>,
  laidOver: boolean,
): void {
  useEffect(() => {
    const over = head.current;
    const holder = pane.current;
    if (!laidOver || over === null || holder === null) {
      return doNothing;
    }
    return scrollWhileHeldOver(over, holder);
  }, [head, pane, laidOver]);
}
