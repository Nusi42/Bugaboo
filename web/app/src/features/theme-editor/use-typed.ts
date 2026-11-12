import { useState } from "react";

import type { ThemeValues } from "./themes.ts";

/** What the person typed into a field of the editor that `bb-core` did not take yet. */
export interface Typed {
  /** The text as typed while it does not count; `null` while the field shows the theme's value. */
  readonly typed: string | null;
  /** The person typed `text`, which makes `next` of the values (`null`: no values at all). */
  readonly offer: (text: string, next: ThemeValues | null) => void;
  /** Goes back to showing the theme's value. */
  readonly forget: () => void;
}

/**
 * A field of the editor: the text stays on screen as typed, and it becomes the theme's value once
 * `bb-core` takes what it makes of it (the rule a save is judged by).
 */
export function useTyped(
  accepts: (values: ThemeValues) => boolean,
  onChange: (values: ThemeValues) => void,
): Typed {
  const [typed, setTyped] = useState<string | null>(null);
  return {
    typed,
    offer: (text, next) => {
      const taken = next !== null && accepts(next);
      setTyped(taken ? null : text);
      if (next !== null && taken) {
        onChange(next);
      }
    },
    forget: () => {
      setTyped(null);
    },
  };
}
