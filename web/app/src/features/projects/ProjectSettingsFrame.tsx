import { t } from "@bugaboo/messages";
import { useLayout, partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { projectPageAddress } from "../../shared/project-pages.ts";
import { TopicList } from "../shell/index.ts";
import { tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * A page of *Project settings* with the menu of the settings pages: the menu of the other settings
 * screens (`TopicList`), at the side on a computer and above the page on a phone. Each page is
 * listed only for people who may use it; with one page, no menu is drawn.
 */
export function ProjectSettingsFrame({
  project,
  children,
}: {
  readonly project: OneProject;
  readonly children: ReactNode;
}) {
  const pages = tabsOf(project);
  const { pathname } = useLocation();
  const phone = useLayout() === "phone";
  const topics = pages.map((page) => ({
    path: projectPageAddress(page, project.project.short_name),
    name: page.tab ?? page.name,
    icon: page.icon,
  }));
  const content = <div {...partOf(part.stack)}>{children}</div>;
  if (topics.length < 2) {
    return content;
  }
  const menu = <TopicList topics={topics} pathname={pathname} />;
  return phone ? (
    <div {...partOf(part.stack)}>
      <nav aria-label={t("projects.tabs")}>{menu}</nav>
      {content}
    </div>
  ) : (
    <div {...partOf(part.settingsFrame)}>
      <nav aria-label={t("projects.tabs")} {...partOf(part.settingsFrameMenu)}>
        {menu}
      </nav>
      <div {...partOf(part.shellContent)}>{content}</div>
    </div>
  );
}
