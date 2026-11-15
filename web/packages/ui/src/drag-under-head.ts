import { useEffect, type RefObject } from "react";

import { doNothing } from "./do-nothing.ts";
import { scrollStepOf, stillHeld } from "./drag-speed.ts";
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

/** A drag that left a part: where it went (nothing when it left the window), and where the pointer is. */
interface DragLeave {
  readonly relatedTarget: unknown;
  readonly clientX: number;
  readonly clientY: number;
}

/**
 * Whether a drag left the window: it went to no part, and the pointer is at or past an edge of the window (WebKit tells
 * of no part on every leave, also inside the window).
 */
function leftTheWindow(event: DragLeave): boolean {
  const { clientX: x, clientY: y } = event;
  return event.relatedTarget === null && (x <= 0 || y <= 0 || x >= innerWidth || y >= innerHeight);
}

/** How deep the point is in the head: 0 at its lower edge, 1 at its top. */
function depthIn(head: HTMLElement, point: Point): number {
  const place = head.getBoundingClientRect();
  return place.height > 0 ? (place.bottom - point.y) / place.height : 0;
}

/**
 * Scrolls the box under the pointer while a drag is held over the head, until it leaves the head, leaves the window or
 * ends, or the browser stops telling of it.
 */
class HeldOverHead {
  private held: Point | null = null;
  private toldAt = 0;
  private frame = 0;
  private last = 0;
  /** The part of a pixel not scrolled yet: the box scrolls by whole pixels. */
  private owed = 0;

  private readonly head: HTMLElement;
  private readonly pane: HTMLElement;

  constructor(head: HTMLElement, pane: HTMLElement) {
    this.head = head;
    this.pane = pane;
  }

  /** A drag is over the head, at this point. */
  readonly over = (event: DragOver): void => {
    this.held = { x: event.clientX, y: event.clientY };
    this.toldAt = performance.now();
    if (this.frame === 0) {
      document.addEventListener("dragover", this.elsewhere, true);
      document.addEventListener("dragleave", this.left, true);
      document.addEventListener("drop", this.stop, true);
      document.addEventListener("dragend", this.stop, true);
      this.frame = requestAnimationFrame(this.scroll);
    }
  };

  readonly stop = (): void => {
    this.held = null;
    cancelAnimationFrame(this.frame);
    [this.frame, this.last, this.owed] = [0, 0, 0];
    document.removeEventListener("dragover", this.elsewhere, true);
    document.removeEventListener("dragleave", this.left, true);
    document.removeEventListener("drop", this.stop, true);
    document.removeEventListener("dragend", this.stop, true);
  };

  private readonly scroll = (now: number): void => {
    const { held } = this;
    if (held === null || !stillHeld(this.toldAt, now)) {
      this.stop();
      return;
    }
    this.owed += scrollStepOf(depthIn(this.head, held), this.last === 0 ? 0 : now - this.last);
    this.last = now;
    const whole = Math.floor(this.owed);
    this.owed -= whole;
    const box = boxUnder(this.pane, this.head, held);
    if (box !== null && whole > 0) {
      box.scrollTop -= whole;
    }
    this.frame = requestAnimationFrame(this.scroll);
  };

  private readonly elsewhere = (event: DragOver): void => {
    this.toldAt = performance.now();
    if (!(event.target instanceof Node) || !this.head.contains(event.target)) {
      this.stop();
    }
  };

  private readonly left = (event: DragLeave): void => {
    if (leftTheWindow(event)) {
      this.stop();
    }
  };
}

/** Scrolls the box under the pointer while a drag is held over `head`; gives back what stops listening. */
function scrollWhileHeldOver(head: HTMLElement, pane: HTMLElement): () => void {
  const held = new HeldOverHead(head, pane);
  head.addEventListener("dragover", held.over);
  return () => {
    held.stop();
    head.removeEventListener("dragover", held.over);
  };
}

/**
 * While a dragged thing is held over the head of a pane that is laid over its boxes, the box under the pointer scrolls
 * up, as a box scrolls when a drag comes near its top edge: the head stands where that edge is, and the browser does
 * not scroll a box for a pointer over another part (UX §6.5). The deeper the pointer is in the head, the faster the box
 * scrolls (`drag-speed.ts`); the drag leaving the head or the window, a drop, the end of the drag or the browser no
 * longer telling of it stops it.
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
