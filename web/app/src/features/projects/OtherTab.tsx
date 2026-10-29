import { useLayout } from "@bugaboo/ui";
import { Navigate, useLocation } from "@tanstack/react-router";

import { asksForSettingsMenu } from "../../shared/paths.ts";
import { projectPageAddress } from "../../shared/project-pages.ts";
import { NotFoundPage } from "../shell/index.ts";
import { ProjectSettingsFrame } from "./ProjectSettingsFrame.tsx";
import { tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * For a person who may do neither of the two: the first other tab they may open, else nothing. On a phone
 * that asks for the menu of the settings, the menu is shown whatever the person may open, as for anybody.
 */
export function OtherTab({ project }: { readonly project: OneProject }) {
  const phone = useLayout() === "phone";
  const { searchStr } = useLocation();
  const tabs = tabsOf(project);
  const [first] = tabs;
  if (first === undefined) {
    return <NotFoundPage />;
  }
  if (phone && tabs.length > 1 && asksForSettingsMenu(searchStr)) {
    return <ProjectSettingsFrame project={project} />;
  }
  return <Navigate to={projectPageAddress(first, project.project.short_name)} replace />;
}
