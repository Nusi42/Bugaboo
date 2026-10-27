import { t } from "@bugaboo/messages";
import { Overlay, overlayEdge, useLayout } from "@bugaboo/ui";
import { lazy, Suspense } from "react";

import { useOverlay } from "./overlay-state.ts";

/** The inside of the dialog is loaded when it is first opened, so the first page stays small (M2-F18). */
const QuickSearchBox = lazy(async () => ({
  default: (await import("./QuickSearchBox.tsx")).QuickSearchBox,
}));

/**
 * Quick search (UX §6.2, CFG-08): `Ctrl+K`, the search box or the button opens a dialog with one
 * box and a list of what was found: the pages of the app now, and whatever other features add as
 * sources (projects, issues, saved searches and actions). Arrow keys move through the list, Enter
 * opens, and Esc closes and puts the focus back where it was.
 */
export function QuickSearch() {
  const overlay = useOverlay("search");
  const phone = useLayout() === "phone";
  return (
    <Overlay
      {...overlay}
      label={t("search.title")}
      from={phone ? overlayEdge.top : overlayEdge.center}
    >
      {overlay.isOpen ? (
        <Suspense fallback={null}>
          <QuickSearchBox />
        </Suspense>
      ) : null}
    </Overlay>
  );
}
