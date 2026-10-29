import { Navigate } from "@tanstack/react-router";

import { projectPageAddress } from "../../shared/project-pages.ts";
import { NotFoundPage } from "../shell/index.ts";
import { ProjectSettingsFrame } from "./ProjectSettingsFrame.tsx";
import { tabsOf } from "./settings-tabs.ts";
import { useShowsSettingsMenu } from "./use-settings-menu.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * For a person who may do neither of the two: the first other tab they may open, else nothing. On a phone
 * that asks for the menu of the settings, the menu is shown whatever the person may open, as for anybody.
 */
export function OtherTab({ project }: { readonly project: OneProject }) {
  const showsMenu = useShowsSettingsMenu(project);
  const [first] = tabsOf(project);
  if (first === undefined) {
    return <NotFoundPage />;
  }
  if (showsMenu) {
    return <ProjectSettingsFrame project={project} />;
  }
  return <Navigate to={projectPageAddress(first, project.project.short_name)} replace />;
}
