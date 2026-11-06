import { useEffect, useState } from "react";

import { isTyping } from "./shortcuts-engine.ts";

/**
 * Whether the person is typing: the focus is in a field. What floats over the page steps aside
 * while it is true, so it never covers the field that is typed in. While a button or the pointer
 * is pressed it does not come back: the press that takes the focus out of the field would end
 * on what floats there, and the button would not be pressed.
 */
export function useTyping(): boolean {
  const [typing, setTyping] = useState(() => isTyping(document.activeElement));
  useEffect(() => {
    let pressed = false;
    const look = () => {
      if (!pressed) {
        setTyping(isTyping(document.activeElement));
      }
    };
    const press = () => {
      pressed = true;
    };
    const release = () => {
      pressed = false;
      // After the click that the release makes, not before it.
      setTimeout(look, 0);
    };
    document.addEventListener("focusin", look);
    document.addEventListener("focusout", look);
    document.addEventListener("pointerdown", press, true);
    document.addEventListener("pointerup", release, true);
    document.addEventListener("pointercancel", release, true);
    return () => {
      document.removeEventListener("focusin", look);
      document.removeEventListener("focusout", look);
      document.removeEventListener("pointerdown", press, true);
      document.removeEventListener("pointerup", release, true);
      document.removeEventListener("pointercancel", release, true);
    };
  }, []);
  return typing;
}
