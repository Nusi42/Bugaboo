import { useEffect } from "react";

/** The open side closes on *Escape* and on a press outside it, as a menu does. */
export function useCloseOutside(
  strip: { readonly current: HTMLElement | null },
  close: () => void,
): void {
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        close();
      }
    };
    const onPress = (event: { readonly target: unknown }): void => {
      if (event.target instanceof Node && strip.current?.contains(event.target) !== true) {
        close();
      }
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("pointerdown", onPress);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerdown", onPress);
    };
  }, [strip, close]);
}
