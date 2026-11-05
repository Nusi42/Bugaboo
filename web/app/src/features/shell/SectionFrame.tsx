import { t, type MessageKey } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import type { SidebarSectionView } from "./sidebar-sections.ts";

export interface SectionFrameProps {
  /** The heading above the section, and its name for screen readers. */
  readonly heading: MessageKey;
  /** How the part is shown where it stands. */
  readonly view: SidebarSectionView;
  readonly children: ReactNode;
}

/**
 * The frame of a part of the sidebar or of the sheet *More*: its name for screen readers and its
 * heading, unless the sidebar is narrowed to icons. A part draws the frame itself, so it can leave
 * itself out, heading and all, when it has nothing to show (M5-F150).
 */
export function SectionFrame({ heading, view, children }: SectionFrameProps) {
  if (view.place === "sheet") {
    return (
      <section {...partOf(part.column)} aria-label={t(heading)}>
        <h3 {...partOf(part.shellSectionHeading)}>{t(heading)}</h3>
        {children}
      </section>
    );
  }
  return (
    <section {...partOf(part.sidebarSection)} aria-label={t(heading)}>
      {view.narrow ? null : <h2 {...partOf(part.shellSectionHeading)}>{t(heading)}</h2>}
      {children}
    </section>
  );
}
