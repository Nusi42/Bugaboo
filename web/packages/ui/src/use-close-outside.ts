import { useEffect, useLayoutEffect, useRef } from "react";

/**
 * The open side closes on *Escape* (`onEscape`, which also moves the focus) and on a press outside it (`close`), as a
 * menu does. The latest handlers are kept in a ref, so the listeners stay the same while the row is drawn again.
 */
export function useCloseOutside(
  strip: { readonly current: HTMLElement | null },
  close: () => void,
  onEscape: () => void,
): void {
  const latest = useRef({ close, onEscape });
  useLayoutEffect(() => {
    latest.current = { close, onEscape };
  });
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        latest.current.onEscape();
      }
    };
    const onPress = (event: { readonly target: unknown }): void => {
      if (event.target instanceof Node && strip.current?.contains(event.target) !== true) {
        latest.current.close();
      }
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("pointerdown", onPress);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerdown", onPress);
    };
  }, [strip]);
}
