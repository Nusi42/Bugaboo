import { useEffect, useRef, useState } from "react";

import { skeletonDelayMs, useAfterDelay } from "./after-delay.ts";
import { doNothing } from "./do-nothing.ts";
import { hiddenPresence, presenceAfter, type WindowPresence } from "./window-shape.ts";

/** What a window that waits hands over to the next window of the same form, which takes its place. */
export interface Handover {
  /** The form of the window, as the settings name it: only a window of the same form takes over. */
  readonly form: string;
  readonly presence: WindowPresence;
  /** What had the focus before the first of them opened, where it goes back when the window closes. */
  readonly returnTo: HTMLElement | null;
  /** When the first of them began to wait (`performance.now()`), so the 0.3 s are counted once. */
  readonly since: number;
}

/**
 * The window that waits now, if any. A window whose code is fetched when it is first opened is stood
 * in for by the same window, waiting (`WaitingWindow` of the app); when the code arrives the real
 * window takes its place and starts where the stand-in was: grey shapes that showed stay shown at the
 * same top, a stand-in that was still clear lets the real window wait only for what is left of the
 * 0.3 s, and the focus goes back to where it was before the stand-in opened (the stand-in is gone by
 * then, and it was what the real window found focused when it came).
 */
let waitingNow: Handover | null = null;

/** Offers this window's place to the next window of its form. */
export function offerHandover(handover: Handover): void {
  waitingNow = handover;
}

/** Takes back this window's offer, unless another window has made one since. */
export function withdrawHandover(mine: Handover): void {
  if (waitingNow === mine) {
    waitingNow = null;
  }
}

/** The place a window of this form takes over, if one waits for it. */
export function handoverFor(form: string | undefined): Handover | null {
  return form !== undefined && waitingNow?.form === form ? waitingNow : null;
}

/** What is left of the wait before grey shapes show, for a wait that began at `since`. */
export function delayLeft(since: number, now: number): number {
  return Math.max(0, skeletonDelayMs - (now - since));
}

/** What has the focus now, if it is an element that can take it back. */
function focusedNow(): HTMLElement | null {
  return typeof document !== "undefined" && document.activeElement instanceof HTMLElement
    ? document.activeElement
    : null;
}

/** How the window began: what it took over, what had the focus, and how long it waits before grey shapes. */
function startOf(form: string | undefined) {
  const handed = handoverFor(form);
  const delay = handed === null ? skeletonDelayMs : delayLeft(handed.since, performance.now());
  return { handed, focused: focusedNow(), delay };
}

/**
 * What the window shows yet and where it stands (`presenceAfter`), and whether its content is late.
 * A window of a `form` hands over to the next window of the same form while it waits.
 */
export function usePresence(
  isOpen: boolean,
  ready: boolean,
  form: string | undefined,
): { readonly presence: WindowPresence; readonly late: boolean } {
  const [start] = useState(() => startOf(form));
  const [presence, setPresence] = useState(start.handed?.presence ?? hiddenPresence);
  const late = useAfterDelay(isOpen && !presence.shown, start.delay);
  const next = presenceAfter(presence, { isOpen, ready, late });
  if (next.shown !== presence.shown || next.anchored !== presence.anchored) {
    setPresence(next);
  }
  const waits = isOpen && !next.shown;
  const { anchored } = next;
  const began = useRef<number | null>(null);
  useEffect(() => {
    if (!isOpen) {
      began.current = null;
    }
    if (!waits || form === undefined) {
      return doNothing;
    }
    // The wait began when this window began to wait, or earlier in the window it took over from.
    began.current ??= start.handed?.since ?? performance.now();
    const mine: Handover = {
      form,
      presence: { shown: false, anchored },
      returnTo: start.handed?.returnTo ?? start.focused,
      since: began.current,
    };
    offerHandover(mine);
    return () => {
      withdrawHandover(mine);
    };
  }, [isOpen, waits, anchored, form, start]);
  useFocusBack(isOpen, start.handed?.returnTo ?? null);
  return { presence: next, late };
}

/**
 * When a window that took over from a stand-in closes, its focus scope puts the focus back where it
 * found it: the page, once the stand-in was gone. Then the focus goes to what opened the stand-in,
 * unless something else has it.
 */
function useFocusBack(isOpen: boolean, returnTo: HTMLElement | null): void {
  useEffect(() => {
    if (!isOpen || returnTo === null) {
      return doNothing;
    }
    return () => {
      requestAnimationFrame(() => {
        if (document.activeElement === document.body && returnTo.isConnected) {
          returnTo.focus();
        }
      });
    };
  }, [isOpen, returnTo]);
}
