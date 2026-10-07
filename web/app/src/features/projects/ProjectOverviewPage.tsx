import { Card } from "@bugaboo/ui";

import { ProjectFacts } from "./ProjectFacts.tsx";
import { ProjectGate } from "./ProjectGate.tsx";
import styles from "./Projects.module.css";
import { SubProjects } from "./SubProjects.tsx";
import { projectPage } from "./use-project-page.ts";
import type { OneProject } from "./use-projects.ts";

/** The page itself, once the project is known. */
function Overview({ project }: { readonly project: OneProject }) {
  return (
    <div className={styles.stack}>
      <Card heading={project.project.name} level={1}>
        <ProjectFacts project={project} />
      </Card>
      <SubProjects project={project} />
    </div>
  );
}

/**
 * The overview of a project, `/p/WEB` (UX §2.1): its name, short name and where it is, its
 * sub-projects, and the way to its settings for those who may change them. An old short name opens
 * the project and the address is replaced by the current one. A project the person may not see is
 * not found, like one that does not exist.
 */
export function ProjectOverviewPage({ address }: { readonly address: string }) {
  return (
    <ProjectGate address={address} kind={projectPage.overview}>
      {(project) => <Overview project={project} />}
    </ProjectGate>
  );
}
