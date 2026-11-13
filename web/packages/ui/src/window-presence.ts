import { useEffect, useState } from "react";

import { useAfterDelay } from "./after-delay.ts";
import { doNothing } from "./do-nothing.ts";
import { hiddenPresence, presenceAfter, type WindowPresence } from "./window-shape.ts";

/** What a window that waits hands over to the window that takes its place. */
interface Handover {
  readonly presence: WindowPresence;
  /** What had the focus before the first of them opened, where it goes back when the window closes. */
  readonly returnTo: HTMLElement | null;
}

const nothingHanded: Handover = { presence: hiddenPresence, returnTo: null };

/**
 * How far the window that waits now has got. A window whose code is fetched when it is first opened
 * is stood in for by the same window, waiting (`WaitingWindow` of the app); when the code arrives the
 * real window takes its place and starts where the stand-in was, so grey shapes that showed stay
 * shown, at the same top, instead of going clear for another 0.3 s or back to the middle. The focus
 * goes back to where it was before the stand-in opened: the stand-in is gone by then, and it was what
 * the real window found focused when it came.
 */
let waitingNow: Handover = nothingHanded;

/** What has the focus now, if it is an element that can take it back. */
function focusedNow(): HTMLElement | null {
  return typeof document !== "undefined" && document.activeElement instanceof HTMLElement
    ? document.activeElement
    : null;
}

/** What the window shows yet and where it stands (`presenceAfter`), and whether its content is late. */
export function usePresence(
  isOpen: boolean,
  ready: boolean,
): { readonly presence: WindowPresence; readonly late: boolean } {
  const [start] = useState(() => ({ handed: waitingNow, focused: focusedNow() }));
  const [presence, setPresence] = useState(start.handed.presence);
  const late = useAfterDelay(isOpen && !presence.shown);
  const next = presenceAfter(presence, { isOpen, ready, late });
  if (next.shown !== presence.shown || next.anchored !== presence.anchored) {
    setPresence(next);
  }
  const waits = isOpen && !next.shown;
  const { anchored } = next;
  useEffect(() => {
    if (!waits) {
      return doNothing;
    }
    const returnTo = start.handed.returnTo ?? start.focused;
    waitingNow = { presence: { shown: false, anchored }, returnTo };
    return () => {
      waitingNow = nothingHanded;
    };
  }, [waits, anchored, start]);
  useEffect(() => {
    const { returnTo } = start.handed;
    if (!isOpen || returnTo === null) {
      return doNothing;
    }
    // After the focus scope of the window has put the focus back where it found it (the page, once
    // the stand-in was gone), the focus goes to what opened the stand-in, unless something else has it.
    return () => {
      requestAnimationFrame(() => {
        if (document.activeElement === document.body && returnTo.isConnected) {
          returnTo.focus();
        }
      });
    };
  }, [isOpen, start]);
  return { presence: next, late };
}
