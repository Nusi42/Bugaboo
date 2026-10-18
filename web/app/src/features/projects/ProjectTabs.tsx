import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { Link } from "react-aria-components";

import { projectPageAddress } from "../../shared/project-pages.ts";
import { tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/** The value of `aria-current` for the page that is open: a word of the ARIA standard. */
const currentPage = "page";

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
      <ul {...partOf(part.projectsTabs)}>
        {tabs.map((tab) => {
          const address = projectPageAddress(tab, shortName);
          return (
            <li key={tab.path}>
              <Link
                href={address}
                {...partOf(part.projectsTab)}
                {...(pathname === address ? { "aria-current": currentPage } : {})}
              >
                {t(tab.tab ?? tab.name)}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
