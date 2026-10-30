import {
  mayOpenProjectPage,
  projectPageAddress,
  settingsTabs,
  type ProjectPage,
} from "../../shared/project-pages.ts";
import type { OneProject } from "./use-projects.ts";

/** Which tabs of the settings the person may open in this project, in the order they are drawn. */
export function tabsOf(project: OneProject): readonly ProjectPage[] {
  return settingsTabs.filter((tab) => mayOpenProjectPage(tab, project));
}

/** The topics of the settings of a project, as the menu of every settings screen lists them. */
export function projectTopics(project: OneProject) {
  return tabsOf(project).map((page) => ({
    path: projectPageAddress(page, project.project.short_name),
    name: page.tab ?? page.name,
    icon: page.icon,
  }));
}
