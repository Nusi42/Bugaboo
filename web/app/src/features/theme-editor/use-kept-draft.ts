import { useEffect, useState } from "react";

import { sameValues } from "./model.ts";
import { keptValues, type ThemeValues } from "./themes.ts";

/** The values of a draft as the editor kept them; anything else is no values. */
function valuesIn(text: string | null): ThemeValues | null {
  if (text === null) {
    return null;
  }
  try {
    const parsed: unknown = JSON.parse(text);
    return typeof parsed === "object" && parsed !== null && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

/** Whether the editor started from a draft the person left, and how to go back to the saved theme. */
export interface KeptDraft {
  readonly kept: boolean;
  readonly drop: () => void;
}

/** Starts the editor from the person's draft of the theme, when one was left that differs from the saved theme. */
export function useKeptDraft(
  theme: string,
  saved: ThemeValues,
  put: (values: ThemeValues) => void,
): KeptDraft {
  const [kept, setKept] = useState(false);
  useEffect(() => {
    let live = true;
    void keptValues(theme).then(
      (text) => {
        const found = valuesIn(text);
        if (live && found !== null && !sameValues(found, saved)) {
          setKept(true);
          put(found);
        }
      },
      () => {
        // No draft could be read: the editor starts from the saved theme.
      },
    );
    return () => {
      live = false;
    };
  }, [theme, saved, put]);
  return {
    kept,
    drop: () => {
      setKept(false);
      put(saved);
    },
  };
}
