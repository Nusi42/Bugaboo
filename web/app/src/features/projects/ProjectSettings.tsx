import { t } from "@bugaboo/messages";
import { Card, Link } from "@bugaboo/ui";
import { Navigate, useNavigate } from "@tanstack/react-router";

import { paths, projectAddress, projectRolesAddress } from "../../shared/paths.ts";
import { NotFoundPage } from "../shell/index.ts";
import { ChangeProjectForm } from "./ChangeProjectForm.tsx";
import { MoveProject } from "./MoveProject.tsx";
import { ProjectTabs } from "./ProjectTabs.tsx";
import styles from "./Projects.module.css";
import type { OneProject } from "./use-projects.ts";

/** For a person who may do neither of the two: the page of the roles if they decide there, else nothing. */
function OnlyRoles({ project }: { readonly project: OneProject }) {
  return project.permissions.includes("decide-roles") ? (
    <Navigate to={projectRolesAddress(project.project.short_name)} replace />
  ) : (
    <NotFoundPage />
  );
}

/** The settings once the project is known: the name and short name, and *Move*. */
export function ProjectSettings({
  project,
  address,
}: {
  readonly project: OneProject;
  readonly address: string;
}) {
  const navigate = useNavigate();
  const mayChange = project.permissions.includes("change-project-settings");
  const mayMove = project.may_move !== "nowhere";
  const { id, name, short_name: shortName } = project.project;
  if (!mayChange && !mayMove) {
    return <OnlyRoles project={project} />;
  }
  return (
    <div className={styles.stack}>
      <ProjectTabs project={project} />
      <Card heading={t("projects.settings-title", { name })} level={1}>
        <div className={styles.row}>
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
                  params: { project: saved },
                  replace: true,
                });
              }
            }}
          />
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
