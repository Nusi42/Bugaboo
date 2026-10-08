import { t } from "@bugaboo/messages";
import { Card, Link } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { paths, projectAddress, routeParam } from "../../shared/paths.ts";
import { ChangeProjectForm } from "./ChangeProjectForm.tsx";
import { MoveProject } from "./MoveProject.tsx";
import { OtherTab } from "./OtherTab.tsx";
import { ProjectTabs } from "./ProjectTabs.tsx";
import styles from "./Projects.module.css";
import type { OneProject } from "./use-projects.ts";

/** The settings once the project is known: the name and short name, and *Move*, below the tabs. */
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
    return <OtherTab project={project} />;
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
                  params: { [routeParam.project]: saved },
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
