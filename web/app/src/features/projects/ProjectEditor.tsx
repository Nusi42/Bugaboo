import { t } from "@bugaboo/messages";
import { Alert, Link } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { projectAddress } from "../../shared/paths.ts";
import { projectPageAddress, projectPages } from "../../shared/project-pages.ts";
import { ChangeProjectForm } from "./ChangeProjectForm.tsx";
import styles from "./Projects.module.css";
import { useProject } from "./use-projects.ts";

/** The editor of one project in the side panel: its name and short name, and the way to its pages. */
export function ProjectEditor({ shortName }: { readonly shortName: string }) {
  const project = useProject(shortName);
  if (project.isError) {
    return <Alert danger>{messageOf(project.error)}</Alert>;
  }
  if (project.isPending) {
    return <p className={styles.muted}>{t("common.loading")}</p>;
  }
  const changer = project.data.permissions.includes("change-project-settings");
  return (
    <>
      {changer ? <ChangeProjectForm key={project.data.project.id} project={project.data} /> : null}
      <div className={styles.row}>
        <Link href={projectAddress(shortName)}>{t("projects.open")}</Link>
        <Link href={projectPageAddress(projectPages.settings, shortName)}>
          {t("projects.settings-link")}
        </Link>
      </div>
    </>
  );
}
