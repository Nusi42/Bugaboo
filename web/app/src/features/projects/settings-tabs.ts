import {
  mayOpenProjectPage,
  projectPages,
  settingsTabs,
  type ProjectPage,
} from "../../shared/project-pages.ts";
import type { OneProject } from "./use-projects.ts";

/** Whether the person may open a tab in this project. */
function mayOpenTab(project: OneProject, tab: ProjectPage): boolean {
  if (tab === projectPages.settings) {
    // Not one permission alone: changing the project, or moving it somewhere.
    return (
      project.permissions.includes("change-project-settings") || project.may_move !== "nowhere"
    );
  }
  return mayOpenProjectPage(tab, project.permissions);
}

/** Which tabs of the settings the person may open in this project, in the order they are drawn. */
export function tabsOf(project: OneProject): readonly ProjectPage[] {
  return settingsTabs.filter((tab) => mayOpenTab(project, tab));
}
