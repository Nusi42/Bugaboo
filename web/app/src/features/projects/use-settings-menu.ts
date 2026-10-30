import { useLayout } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import { asksForSettingsMenu, projectSettingsMenuAddress } from "../../shared/paths.ts";
import { projectPageAddress, projectPages, type ProjectPage } from "../../shared/project-pages.ts";
import { tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/** A menu of the settings is drawn for two pages or more; with one page there is nothing to choose. */
export function hasSettingsMenu(tabCount: number): boolean {
  return tabCount > 1;
}

/**
 * Whether the settings of a project show their menu as the screen: a phone that asks for it (`?menu`),
 * where there is a menu. The one rule; the frame, the trail and the page that is turned away read it.
 */
export function showsSettingsMenu(phone: boolean, tabCount: number, searchStr: string): boolean {
  return phone && hasSettingsMenu(tabCount) && asksForSettingsMenu(searchStr);
}

/**
 * Where the settings of a project start: the menu where a phone has one, else the first page the person may
 * open (the settings page itself where none is named). The one place that says so, for the links and the trail.
 */
export function settingsStartAddress(
  shortName: string,
  hasMenu: boolean,
  first: ProjectPage = projectPages.settings,
): string {
  return hasMenu ? projectSettingsMenuAddress(shortName) : projectPageAddress(first, shortName);
}

/**
 * What a phone does with the menu of the settings of the project that is open (none while it loads): whether
 * there is one, and whether it is the screen now.
 */
export function useSettingsMenu(project: OneProject | undefined): {
  readonly hasMenu: boolean;
  readonly onMenu: boolean;
} {
  const phone = useLayout() === "phone";
  const { searchStr } = useLocation();
  const tabCount = project === undefined ? 0 : tabsOf(project).length;
  return {
    hasMenu: phone && hasSettingsMenu(tabCount),
    onMenu: showsSettingsMenu(phone, tabCount, searchStr),
  };
}

/** Whether the menu is the screen now, for the project whose settings are open. */
export function useShowsSettingsMenu(project: OneProject): boolean {
  return useSettingsMenu(project).onMenu;
}
