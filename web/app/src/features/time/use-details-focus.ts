import { focusFirstField, withId } from "@bugaboo/ui";
import { useEffect, useRef } from "react";

import { entryItemId } from "./entry-ids.ts";

/**
 * Where the focus goes when the list of the entries is swapped for the form of one and back (WCAG
 * 2.4.3): to the first box of the form, and from the form to the button of the entry that was
 * changed. `picked` is the ID of the entry whose form is open, if one is.
 */
export function useDetailsFocus(formId: string, picked: string | undefined): void {
  const last = useRef<string | null>(null);
  useEffect(() => {
    if (picked !== undefined) {
      last.current = picked;
      focusFirstField(formId);
    } else if (last.current !== null) {
      document
        .querySelector(withId(entryItemId(last.current)))
        ?.querySelector<HTMLElement>("button")
        ?.focus();
    }
  }, [formId, picked]);
}
