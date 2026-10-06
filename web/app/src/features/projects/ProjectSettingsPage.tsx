import type { ReactNode } from "react";

import { ProjectGate } from "./ProjectGate.tsx";
import { ProjectSettings } from "./ProjectSettings.tsx";
import { projectPage } from "./use-project-page.ts";
import type { OneProject } from "./use-projects.ts";

/**
 * *Project settings*, `/p/WEB/settings` (PRJ-01, PRJ-03): the name and short name of a project and
 * *Move*, for the people who may change them, normally its project leads. A person who may do
 * neither finds nothing here, like at an address that does not exist.
 */
export function ProjectSettingsPage({
  address,
  extra,
}: {
  readonly address: string;
  /** More cards for the page, from features that sit beside this one (what roles see here). */
  readonly extra?: (project: OneProject) => ReactNode;
}) {
  return (
    <ProjectGate address={address} kind={projectPage.settings}>
      {(project) => (
        <ProjectSettings project={project} address={address} extra={extra?.(project)} />
      )}
    </ProjectGate>
  );
}
