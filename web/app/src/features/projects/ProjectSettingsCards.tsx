import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { TimeDefaultsOfProject } from "../fields/index.ts";
import { ProjectSettingsSection } from "../settings/index.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * The cards of a project's settings that the people who may change the project see: its own
 * settings (*Issue IDs* and the rest of the registry) and the defaults of the fields of time
 * entries.
 */
export function ProjectSettingsCards({ project }: { readonly project: OneProject }) {
  return (
    <>
      <Card heading={t("projects.issue-ids-title")}>
        <ProjectSettingsSection projectId={project.project.id} />
      </Card>
      <TimeDefaultsOfProject project={project} />
    </>
  );
}
