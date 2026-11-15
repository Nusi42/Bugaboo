import { useState } from "react";

/**
 * The text of a box that follows the value it shows. When somebody else's value comes in, a box
 * the person has not touched shows it; a box they are typing in keeps what they typed, so a change
 * from outside never throws their words away.
 */
export function useSynced(shown: string): readonly [string, (next: string) => void] {
  const [text, setText] = useState(shown);
  const [base, setBase] = useState(shown);
  if (base !== shown) {
    setBase(shown);
    if (text === base) {
      setText(shown);
    }
  }
  return [text, setText];
}
