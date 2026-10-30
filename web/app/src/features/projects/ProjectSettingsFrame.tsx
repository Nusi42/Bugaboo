import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { SettingsScreens, TopicList } from "../shell/index.ts";
import { projectTopics } from "./settings-tabs.ts";
import { hasSettingsMenu, useShowsSettingsMenu } from "./use-settings-menu.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * A page of *Project settings*, in the frame of every settings screen (`SettingsScreens`): the menu of
 * the pages at the side on a computer, a screen of its own on a phone (`?menu`). Each page is listed
 * only for people who may use it; with one page, no menu is drawn. The screen of the menu marks no page: its address is
 * the one of the first page, which is not open there.
 */
export function ProjectSettingsFrame({
  project,
  children,
}: {
  readonly project: OneProject;
  readonly children?: ReactNode;
}) {
  const { pathname } = useLocation();
  const topics = projectTopics(project);
  const onMenu = useShowsSettingsMenu(project);
  return (
    <SettingsScreens
      heading={t("projects.settings-title", { name: project.project.name })}
      label={t("projects.tabs")}
      menu={
        hasSettingsMenu(topics.length) ? (
          <TopicList topics={topics} pathname={onMenu ? undefined : pathname} />
        ) : undefined
      }
      onMenu={onMenu}
    >
      <div {...partOf(part.stack)}>{children}</div>
    </SettingsScreens>
  );
}
