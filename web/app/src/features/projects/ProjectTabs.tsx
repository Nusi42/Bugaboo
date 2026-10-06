import { t } from "@bugaboo/messages";
import { Link } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import { projectRolesAddress, projectSettingsAddress } from "../../shared/paths.ts";
import styles from "./Projects.module.css";
import { settingsTab, type SettingsTab } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/** Which tabs of the settings the person may open in this project. */
function tabsOf(project: OneProject): readonly SettingsTab[] {
  const tabs: SettingsTab[] = [];
  if (project.permissions.includes("change-project-settings") || project.may_move !== "nowhere") {
    tabs.push(settingsTab.general);
  }
  if (project.permissions.includes("decide-roles")) {
    tabs.push(settingsTab.roles);
  }
  return tabs;
}

/**
 * The tabs of *Project settings*: the name and place of the project, and *Who has which role* in it.
 * Each is there only for people who may use it; with one tab, nothing is drawn.
 */
export function ProjectTabs({ project }: { readonly project: OneProject }) {
  const tabs = tabsOf(project);
  const { pathname } = useLocation();
  const shortName = project.project.short_name;
  if (tabs.length < 2) {
    return null;
  }
  return (
    <nav aria-label={t("projects.tabs")}>
      <ul className={styles.tabs}>
        {tabs.map((tab) => (
          <li key={tab}>
            <Link
              href={
                tab === settingsTab.general
                  ? projectSettingsAddress(shortName)
                  : projectRolesAddress(shortName)
              }
              current={
                pathname ===
                (tab === settingsTab.general
                  ? projectSettingsAddress(shortName)
                  : projectRolesAddress(shortName))
              }
            >
              {t(tab === settingsTab.general ? "projects.tab-general" : "roles.project-link")}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}
