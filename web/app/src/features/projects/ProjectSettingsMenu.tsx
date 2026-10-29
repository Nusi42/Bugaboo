import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import { TopicList } from "../shell/index.ts";
import { projectTopics } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/** The settings of a project on a phone: the menu is a screen of its own, and each section opens with Back in the bar. */
export function ProjectSettingsMenu({ project }: { readonly project: OneProject }) {
  const { pathname } = useLocation();
  return (
    <Card heading={t("projects.settings-title", { name: project.project.name })} level={1}>
      <nav aria-label={t("projects.tabs")}>
        <TopicList topics={projectTopics(project)} pathname={pathname} />
      </nav>
    </Card>
  );
}
