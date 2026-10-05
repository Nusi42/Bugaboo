import { useEffect, useRef, type RefObject } from "react";

/**
 * Gives a ref for an element; `onHold` is called when the pointer is over it or the focus is in it,
 * and `onRelease` when neither is any more. For things that must wait while somebody looks at them
 * (an offer to undo). The listeners are attached to the element itself, not as props.
 */
export function useHold<Element extends HTMLElement>(
  onHold: () => void,
  onRelease: () => void,
): RefObject<Element | null> {
  const element = useRef<Element>(null);
  const latest = useRef({ onHold, onRelease });
  useEffect(() => {
    latest.current = { onHold, onRelease };
  });
  useEffect(() => {
    const target = element.current;
    let hovered = false;
    let focused = false;
    let held = false;
    const update = (): void => {
      const now = hovered || focused;
      if (now !== held) {
        held = now;
        if (now) {
          latest.current.onHold();
        } else {
          latest.current.onRelease();
        }
      }
    };
    const set = (what: "hover" | "focus", on: boolean) => () => {
      if (what === "hover") {
        hovered = on;
      } else {
        focused = on;
      }
      update();
    };
    const handlers = [
      ["mouseenter", set("hover", true)],
      ["mouseleave", set("hover", false)],
      ["focusin", set("focus", true)],
      ["focusout", set("focus", false)],
    ] as const;
    for (const [name, handler] of handlers) {
      target?.addEventListener(name, handler);
    }
    return () => {
      for (const [name, handler] of handlers) {
        target?.removeEventListener(name, handler);
      }
    };
  }, []);
  return element;
}
