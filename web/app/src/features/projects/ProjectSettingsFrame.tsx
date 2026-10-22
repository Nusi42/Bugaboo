import { t } from "@bugaboo/messages";
import { useLayout, partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { Link } from "react-aria-components";

import { projectPageAddress } from "../../shared/project-pages.ts";
import { tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/** The value of `aria-current` for the page that is open: a word of the ARIA standard. */
const currentPage = "page";

/**
 * A page of *Project settings* with the list of the pages at its side (above it on a phone): the
 * name and place of the project, *Who has which role* in it, and what roles see there (field
 * permissions, who sees people, *Check access*). Each is listed only for people who may use it;
 * with one page, no list is drawn.
 */
export function ProjectSettingsFrame({
  project,
  children,
}: {
  readonly project: OneProject;
  readonly children: ReactNode;
}) {
  const tabs = tabsOf(project);
  const { pathname } = useLocation();
  const variants = useLayout() === "phone" ? (["phone"] as const) : [];
  const shortName = project.project.short_name;
  const page = (
    <div {...partOf(part.projectsPage)}>
      <div {...partOf(part.stack)}>{children}</div>
    </div>
  );
  if (tabs.length < 2) {
    return page;
  }
  return (
    <div {...partOf(part.projectsSettings, { variants })}>
      <nav aria-label={t("projects.tabs")}>
        <ul {...partOf(part.projectsTabs, { variants })}>
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
      {page}
    </div>
  );
}
