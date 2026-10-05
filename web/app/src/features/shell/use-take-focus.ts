import { useEffect, useRef, type RefObject } from "react";

/**
 * Gives a ref for an element; the focus moves to the element when it appears, and goes back to what
 * had it before when the element goes away. A panel or view that opens from a button is then
 * reachable with the keyboard at once, Esc works inside it, and closing it does not lose the place.
 */
export function useTakeFocus<Element extends HTMLElement>(): RefObject<Element | null> {
  const element = useRef<Element>(null);
  useEffect(() => {
    const before = document.activeElement;
    element.current?.focus();
    return () => {
      // After the next paint: what had the focus may only be shown again once this has gone.
      requestAnimationFrame(() => {
        if (before instanceof HTMLElement && before.isConnected) {
          before.focus();
        }
      });
    };
  }, []);
  return element;
}
