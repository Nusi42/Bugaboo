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
  /** What the section shows; links inside it are links of the app. */
  readonly render: () => ReactNode;
}

const sections = createRegistry<SidebarSection>();

/** Adds a section to the sidebar; returns the way to take it out again. */
export const addSidebarSection = sections.add;

export function useSidebarSections(): readonly SidebarSection[] {
  return useRegistry(sections);
}
