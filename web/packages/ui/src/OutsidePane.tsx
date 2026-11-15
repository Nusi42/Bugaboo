import type { ReactNode } from "react";

import { OUTSIDE_PANE, PaneHeadContext } from "./pane-head.ts";

/**
 * What a window, a dialog or a menu shows stands outside the pane it was opened from (it is drawn elsewhere, and the
 * context of React passes through to it all the same): its one card keeps its heading, and nothing goes into the head
 * of that pane.
 */
export function OutsidePane({ children }: { readonly children: ReactNode }) {
  return <PaneHeadContext value={OUTSIDE_PANE}>{children}</PaneHeadContext>;
}
