import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";

import { isTyping } from "../../shared/shortcuts-engine.ts";

/** What Esc does in a field inside the element: closes it too, or is left alone. */
export const escapeInField = { close: "close", leave: "leave" } as const;

/**
 * Gives a ref for an element; `onEscape` is called when Esc is pressed inside it. With
 * `inField: escapeInField.leave`, an Esc in a field is left alone: there it cancels what is typed.
 * The handler is attached to the element itself (not as a prop), so the element stays an ordinary
 * region for screen readers.
 */
export function useEscape<Element extends HTMLElement>(
  onEscape: () => void,
  inField: (typeof escapeInField)[keyof typeof escapeInField] = escapeInField.close,
): RefObject<Element | null> {
  const region = useRef<Element>(null);
  const latest = useRef(onEscape);
  useLayoutEffect(() => {
    latest.current = onEscape;
  });
  useEffect(() => {
    const element = region.current;
    const onKeyDown = (event: KeyboardEvent): void => {
      const target = event.target instanceof HTMLElement ? event.target : null;
      if (
        event.key !== "Escape" ||
        event.defaultPrevented ||
        (inField === escapeInField.leave && isTyping(target))
      ) {
        return;
      }
      event.stopPropagation();
      latest.current();
    };
    element?.addEventListener("keydown", onKeyDown);
    return () => {
      element?.removeEventListener("keydown", onKeyDown);
    };
  }, [inField]);
  return region;
}
