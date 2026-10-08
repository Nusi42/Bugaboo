import { partOf, part } from "@bugaboo/ui";

import { mayOpenProjectPage, projectPages } from "../../shared/project-pages.ts";
import { AssignmentsSection } from "../roles/index.ts";
import { NotFoundPage } from "../shell/index.ts";
import { ProjectGate } from "./ProjectGate.tsx";
import { ProjectTabs } from "./ProjectTabs.tsx";
import type { OneProject } from "./use-projects.ts";

/** The page once the project is known: the tabs, then *Who has which role* in the project. */
function Roles({ project }: { readonly project: OneProject }) {
  if (!mayOpenProjectPage(projectPages.roles, project)) {
    return <NotFoundPage />;
  }
  const { id, name } = project.project;
  return (
    <div {...partOf(part.stack)}>
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
    <ProjectGate address={address} page={projectPages.roles}>
      {(project) => <Roles project={project} />}
    </ProjectGate>
  );
}
