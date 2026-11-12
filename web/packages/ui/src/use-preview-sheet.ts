import { useEffect, useRef } from "react";

import { linkPreviewSheet } from "./theme.ts";

/**
 * Keeps the style sheet at `href` linked while the component is on the screen, for the samples and
 * the preview of own themes (`linkPreviewSheet`); none for `undefined`. A new address is linked
 * first and the old link goes once the new sheet is there, so the preview never shows the built-in
 * theme for a moment in between.
 */
export function usePreviewSheet(href: string | undefined): void {
  const shown = useRef<(() => void) | null>(null);
  useEffect(() => {
    const before = shown.current;
    if (href === undefined) {
      before?.();
      shown.current = null;
    } else {
      shown.current = linkPreviewSheet(href, () => {
        before?.();
      });
    }
  }, [href]);
  useEffect(
    () => () => {
      shown.current?.();
    },
    [],
  );
}
