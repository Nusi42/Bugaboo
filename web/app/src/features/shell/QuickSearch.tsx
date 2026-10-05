import { t } from "@bugaboo/messages";
import { Overlay, useLayout } from "@bugaboo/ui";

import { useOverlay } from "./overlay-state.ts";
import { QuickSearchBox } from "./QuickSearchBox.tsx";

/**
 * Quick search (UX §6.2, CFG-08): `Ctrl+K`, the search box or the button opens a dialog with one
 * box and a list of what was found: the pages of the app now, and whatever other features add as
 * sources (projects with WP-08, issues and actions in M5). Arrow keys move through the list, Enter
 * opens, and Esc closes and puts the focus back where it was.
 */
export function QuickSearch() {
  const overlay = useOverlay("search");
  const phone = useLayout() === "phone";
  return (
    <Overlay {...overlay} label={t("search.title")} top={phone}>
      {overlay.isOpen ? <QuickSearchBox /> : null}
    </Overlay>
  );
}
