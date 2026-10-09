import { t } from "@bugaboo/messages";
import { Card, Link, partOf, part } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { paths, projectAddress, routeParam } from "../../shared/paths.ts";
import { ProjectSettingsSection } from "../settings/index.ts";
import { ChangeProjectForm } from "./ChangeProjectForm.tsx";
import { MoveProject } from "./MoveProject.tsx";
import { ProjectTabs } from "./ProjectTabs.tsx";
import type { OneProject } from "./use-projects.ts";
import { mayChangeProject, mayMoveProject } from "../../shared/project-pages.ts";

/** The settings once the project is known: the name and short name, and *Move*, below the tabs. */
export function ProjectSettings({
  project,
  address,
}: {
  readonly project: OneProject;
  readonly address: string;
}) {
  const navigate = useNavigate();
  const mayChange = mayChangeProject(project);
  const mayMove = mayMoveProject(project);
  const { id, name, short_name: shortName } = project.project;
  return (
    <div {...partOf(part.stack)}>
      <ProjectTabs project={project} />
      <Card heading={t("projects.settings-title", { name })} level={1}>
        <div {...partOf(part.row)}>
          <Link href={projectAddress(shortName)}>{t("projects.settings-back")}</Link>
        </div>
      </Card>
      {mayChange ? (
        <Card heading={t("projects.edit-title")}>
          <ChangeProjectForm
            key={id}
            project={project}
            onSaved={(saved) => {
              if (saved !== address) {
                void navigate({
                  to: paths.projectSettings,
                  params: { [routeParam.project]: saved },
                  replace: true,
                });
              }
            }}
          />
        </Card>
      ) : null}
      {mayChange ? (
        <Card heading={t("projects.issue-ids-title")}>
          <ProjectSettingsSection projectId={id} />
        </Card>
      ) : null}
      {mayMove ? (
        <Card heading={t("projects.move-title")}>
          <MoveProject project={project} />
        </Card>
      ) : null}
    </div>
  );
}
