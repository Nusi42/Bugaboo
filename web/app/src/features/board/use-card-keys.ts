import { useEffect, type RefObject } from "react";

import { cardKeys, type CardKeyInput } from "./card-keys.ts";

/**
 * Reads the keys of a card where its title and its button have the focus: the list item itself takes
 * none, so the keys are listened for on it (TEST-A11Y, UX §4.6).
 */
export function useCardKeys(
  ref: Readonly<RefObject<HTMLElement | null>>,
  input: Omit<CardKeyInput, "from">,
): void {
  const { card, column, state } = input;
  useEffect(() => {
    const from = ref.current;
    const read = (event: KeyboardEvent): void => {
      if (from !== null) {
        cardKeys(event, { card, column, state, from });
      }
    };
    from?.addEventListener("keydown", read);
    return () => {
      from?.removeEventListener("keydown", read);
    };
  }, [ref, card, column, state]);
}
