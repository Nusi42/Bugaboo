import { t } from "@bugaboo/messages";
import { useLayout, partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { TopicList } from "../shell/index.ts";
import { projectTopics } from "./settings-tabs.ts";
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
  const { pathname } = useLocation();
  const topics = projectTopics(project);
  const phone = useLayout() === "phone";
  const content = <div {...partOf(part.stack)}>{children}</div>;
  // On a phone a section is a screen of its own (the menu is another, `ProjectSettingsMenu`).
  if (topics.length < 2 || phone) {
    return content;
  }
  return (
    <div {...partOf(part.settingsFrame)}>
      <nav aria-label={t("projects.tabs")} {...partOf(part.settingsFrameMenu)}>
        <TopicList topics={topics} pathname={pathname} />
      </nav>
      <div {...partOf(part.shellContent)}>{content}</div>
    </div>
  );
}
