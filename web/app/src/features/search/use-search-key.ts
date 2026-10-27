import { useLayout } from "@bugaboo/ui";
import { useRef } from "react";

import { useShortcuts } from "../../shared/use-shortcuts.ts";

/** The box of Advanced search, and the first button when the bar has no box. */
const boxSelector = "textarea";
const buttonSelector = "button";

/**
 * The key `/` goes to the search bar (UX design section 6.2): to the box in Advanced search, to
 * the first button otherwise. On a phone the bar is one button that opens a sheet, so the key
 * opens it. Gives the reference of the element that holds the bar.
 */
export function useSearchKey(openSheet: () => void) {
  const container = useRef<HTMLDivElement>(null);
  const layout = useLayout();
  useShortcuts({
    "focus-search-bar": () => {
      if (layout === "phone") {
        openSheet();
        return;
      }
      const bar = container.current;
      const target =
        bar?.querySelector<HTMLElement>(boxSelector) ??
        bar?.querySelector<HTMLElement>(buttonSelector);
      target?.focus();
    },
  });
  return container;
}
