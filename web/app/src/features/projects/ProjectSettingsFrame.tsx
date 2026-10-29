import { t } from "@bugaboo/messages";
import { useLayout, partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { asksForSettingsMenu } from "../../shared/paths.ts";
import { SettingsScreens, TopicList } from "../shell/index.ts";
import { projectTopics } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * A page of *Project settings*, in the frame of every settings screen (`SettingsScreens`): the menu of
 * the pages at the side on a computer, a screen of its own on a phone (`?menu`). Each page is listed
 * only for people who may use it; with one page, no menu is drawn.
 */
export function ProjectSettingsFrame({
  project,
  children,
}: {
  readonly project: OneProject;
  readonly children?: ReactNode;
}) {
  const { pathname, searchStr } = useLocation();
  const topics = projectTopics(project);
  const phone = useLayout() === "phone";
  return (
    <SettingsScreens
      heading={t("projects.settings-title", { name: project.project.name })}
      label={t("projects.tabs")}
      menu={topics.length < 2 ? undefined : <TopicList topics={topics} pathname={pathname} />}
      onMenu={phone && topics.length > 1 && asksForSettingsMenu(searchStr)}
    >
      <div {...partOf(part.stack)}>{children}</div>
    </SettingsScreens>
  );
}
