import { AssignmentsSection } from "../roles/index.ts";
import { NotFoundPage } from "../shell/index.ts";
import { ProjectGate } from "./ProjectGate.tsx";
import { ProjectTabs } from "./ProjectTabs.tsx";
import styles from "./Projects.module.css";
import { projectPage } from "./use-project-page.ts";
import type { OneProject } from "./use-projects.ts";

/** The page once the project is known: the tabs, then *Who has which role* in the project. */
function Roles({ project }: { readonly project: OneProject }) {
  if (!project.permissions.includes("decide-roles")) {
    return <NotFoundPage />;
  }
  const { id, name } = project.project;
  return (
    <div className={styles.stack}>
      <ProjectTabs project={project} />
      <AssignmentsSection project={{ id, name }} level={1} />
    </div>
  );
}

/**
 * *Who has which role* in a project, `/p/WEB/settings/roles` (SEC-02, SEC-03): the rows of the
 * project and the rows that come from above, *No access*, and the form that gives a role. Only for
 * people who decide who has which role there; to everybody else the page is not found.
 */
export function ProjectRolesPage({ address }: { readonly address: string }) {
  return (
    <ProjectGate address={address} kind={projectPage.roles}>
      {(project) => <Roles project={project} />}
    </ProjectGate>
  );
}
