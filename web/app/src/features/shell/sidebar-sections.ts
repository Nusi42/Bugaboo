import type { MessageKey } from "@bugaboo/messages";
import type { ReactNode } from "react";

import { createRegistry, useRegistry } from "./registry.ts";

/**
 * A part of the sidebar that a feature fills: the project tree comes from the projects feature
 * (WP-08), below the start page and above *Settings*. Nothing is shown for a section until its
 * feature adds it, so the sidebar lists only what exists.
 */
export interface SidebarSection {
  /** The heading above the section. */
  readonly heading: MessageKey;
  /**
   * What the section shows; links inside it are links of the app. `narrow`: the sidebar shows
   * icons only, so the section should show only what fits.
   */
  readonly render: (view: SidebarSectionView) => ReactNode;
}

/** How the sidebar shows a section. */
export interface SidebarSectionView {
  /** The sidebar is narrowed to icons. */
  readonly narrow: boolean;
  /** Called after a link of the section was used, for a menu or a sheet that closes. */
  readonly onNavigate?: () => void;
}

const sections = createRegistry<SidebarSection>();

/** Adds a section to the sidebar; returns the way to take it out again. */
export const addSidebarSection = sections.add;

export function useSidebarSections(): readonly SidebarSection[] {
  return useRegistry(sections);
}
