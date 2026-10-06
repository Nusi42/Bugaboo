import { t } from "@bugaboo/messages";
import { Link } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import styles from "./Projects.module.css";
import { tabAddress, tabName, tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * The tabs of *Project settings*: the name and place of the project, *Who has which role* in it,
 * and what roles see there (field permissions, who sees people, *Check access*). Each is there
 * only for people who may use it; with one tab, nothing is drawn.
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
        {tabs.map((tab) => {
          const address = tabAddress(tab, shortName);
          return (
            <li key={tab}>
              <Link href={address} current={pathname === address}>
                {t(tabName(tab))}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
