import { mayOpenProjectPage, settingsTabs, type ProjectPage } from "../../shared/project-pages.ts";
import type { OneProject } from "./use-projects.ts";

/** Which tabs of the settings the person may open in this project, in the order they are drawn. */
export function tabsOf(project: OneProject): readonly ProjectPage[] {
  return settingsTabs.filter((tab) => mayOpenProjectPage(tab, project));
}
