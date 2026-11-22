import { useEffect, useState, type RefObject } from "react";

import { firstReachable, TAKES_FOCUS } from "./focusable.ts";

/**
 * The attributes that change whether an element takes the focus or is in reach: the box is judged again when one of
 * them changes, not at every change of a class or a value.
 */
const REACH_ATTRIBUTES = [
  "tabindex",
  "disabled",
  "href",
  "hidden",
  "inert",
  "aria-hidden",
  "contenteditable",
];

/**
 * Whether a box that scrolls has to take the focus itself so the keyboard can scroll it (WCAG 2.1.1): while it holds
 * more than it shows and nothing in it in reach takes the focus, the arrow keys reach it only through the box. While the
 * box has the focus it keeps it, so the focus never falls to the page when something that takes the focus appears in
 * it. It is judged again whenever the box changes its size, what it holds, or loses the focus.
 */
export function useKeyboardScroll(box: Readonly<RefObject<HTMLElement | null>>): boolean {
  const [needed, setNeeded] = useState(false);
  useEffect(() => {
    const shown = box.current;
    const judge = (): void => {
      setNeeded(
        shown !== null &&
          (document.activeElement === shown ||
            (shown.scrollHeight > shown.clientHeight &&
              firstReachable(shown, TAKES_FOCUS) === null)),
      );
    };
    const resized = new ResizeObserver(judge);
    const changed = new MutationObserver(judge);
    if (shown !== null) {
      resized.observe(shown);
      changed.observe(shown, {
        childList: true,
        subtree: true,
        attributeFilter: REACH_ATTRIBUTES,
      });
      shown.addEventListener("blur", judge);
    }
    judge();
    return () => {
      resized.disconnect();
      changed.disconnect();
      shown?.removeEventListener("blur", judge);
    };
  }, [box]);
  return needed;
}
