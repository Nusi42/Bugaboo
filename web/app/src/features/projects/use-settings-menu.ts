import { useLayout } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import { asksForSettingsMenu } from "../../shared/paths.ts";
import { tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * Whether the settings of this project show their menu as the screen: a phone that asks for it
 * (`?menu`), where the person may open two pages or more. The one place that says so.
 */
export function useShowsSettingsMenu(project: OneProject): boolean {
  const phone = useLayout() === "phone";
  const { searchStr } = useLocation();
  return phone && tabsOf(project).length > 1 && asksForSettingsMenu(searchStr);
}
