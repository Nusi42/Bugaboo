import { t } from "@bugaboo/messages";
import { Card, PageBack, useLayout } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { paths, projectAddress, routeParam } from "../../shared/paths.ts";
import { ProjectSettingsSection } from "../settings/index.ts";
import { ChangeProjectForm } from "./ChangeProjectForm.tsx";
import { MoveProject } from "./MoveProject.tsx";
import { ProjectSettingsFrame } from "./ProjectSettingsFrame.tsx";
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
  // On a phone the card has only its heading for screen readers: the bar says it, and Back is in the bar.
  const phone = useLayout() === "phone";
  const mayChange = mayChangeProject(project);
  const mayMove = mayMoveProject(project);
  const { id, name, short_name: shortName } = project.project;
  return (
    <ProjectSettingsFrame project={project}>
      <Card heading={t("projects.settings-title", { name })} level={1}>
        {phone ? null : (
          <PageBack href={projectAddress(shortName)} label={t("projects.settings-back")} />
        )}
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
    </ProjectSettingsFrame>
  );
}
