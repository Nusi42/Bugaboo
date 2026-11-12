import { usePreviewSheet } from "@bugaboo/ui";
import { useEffect, useState } from "react";

import { draftSheetOf, keepValues, type ThemeValues } from "./themes.ts";

/** How long the editor waits after the last key before it keeps the values as the draft. */
const DRAFT_PAUSE_MS = 250;

/**
 * Keeps the values on screen as the person's draft of the theme and links the sheet the server
 * makes from it, so the preview shows them without a style made in the page (ADR-0026, update M8).
 * Nothing reaches other people: the draft is the person's own until *Save*. Gives whether the
 * preview can show the draft yet; `on` is false while the person does not hold the lock.
 */
export function useDraftPreview(
  theme: string,
  values: ThemeValues,
  version: number,
  on: boolean,
): boolean {
  const [round, setRound] = useState(0);
  useEffect(() => {
    let live = on;
    const timer = window.setTimeout(() => {
      if (!live) {
        return;
      }
      void keepValues(theme, values, version).then(
        () => {
          if (live) {
            setRound((before) => before + 1);
          }
        },
        () => {
          // The draft could not be kept: the preview shows the last one, and *Save* still works.
        },
      );
    }, DRAFT_PAUSE_MS);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [theme, values, version, on]);
  usePreviewSheet(round === 0 ? undefined : draftSheetOf(theme, round));
  return round > 0;
}
