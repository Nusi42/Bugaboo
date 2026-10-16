import { t } from "@bugaboo/messages";
import { useAppearance, useLayout, partOf, part, type LayoutMode } from "@bugaboo/ui";

import { CONTENT_HREF } from "./content-id.ts";
import { ShellColumn } from "./ShellColumn.tsx";
import { ShellOverlays } from "./ShellOverlays.tsx";
import { Sidebar } from "./Sidebar.tsx";
import { useSidebarNarrow } from "./sidebar-state.ts";
import { useShell } from "./use-shell.ts";

/** What the frame says about itself: a phone, or a computer with its sidebar wide or narrow (a minimized window stands after it). */
function variantOf(layout: LayoutMode, narrow: boolean) {
  if (layout === "phone") {
    return "phone";
  }
  if (layout === "computer") {
    return narrow ? "side-narrow" : "side";
  }
  return false;
}

/**
 * The frame around the pages of somebody who is signed in (UX §3): on a computer a sidebar and a
 * top bar, on a tablet a menu that slides in, on a phone one screen at a time with a bar of
 * labelled buttons at the bottom. The same pages, the same shortcuts and the same themes in all.
 */
export function AppLayout() {
  const layout = useLayout();
  const { narrow, locked } = useSidebarNarrow();
  const { person, signOutProblem } = useShell();
  useAppearance();
  const computer = layout === "computer";
  return (
    <div {...partOf(part.appLayout, { variants: [variantOf(layout, narrow)] })}>
      <a {...partOf(part.skip)} href={CONTENT_HREF}>
        {t("shell.skip")}
      </a>
      {computer ? <Sidebar narrow={narrow} locked={locked} {...person} /> : null}
      <ShellColumn layout={layout} signOutProblem={signOutProblem} />
      <ShellOverlays layout={layout} {...person} />
    </div>
  );
}
