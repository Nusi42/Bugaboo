import type { WindowSize } from "@bugaboo/ui";

import { changeMySettings } from "./change-my-settings.ts";
import { useMySettings } from "./use-my-settings.ts";
import { changeForSize, sizeOfForm } from "./window-sizes.ts";

export interface WindowSizeOfForm {
  /** The size the person gave the window of this form before; nothing at first. */
  readonly size: WindowSize | undefined;
  /** Remembers the size the window ended with, for this person and this form. */
  readonly remember: (size: WindowSize) => void;
}

/**
 * The size of the window of one form (`Window` takes `size` and `onResized`), kept in the person's
 * settings and not in the browser alone (UX §6.3, ADR-0018): the same size on every device, and
 * nothing left at a shared computer.
 */
export function useWindowSize(form: string): WindowSizeOfForm {
  const settings = useMySettings();
  return {
    size: sizeOfForm(settings, form),
    remember: (size) => {
      void changeMySettings([changeForSize(settings, form, size)]);
    },
  };
}
