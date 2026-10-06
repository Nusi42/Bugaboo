import { ProjectGate } from "./ProjectGate.tsx";
import { ProjectSettings } from "./ProjectSettings.tsx";
import { projectPage } from "./use-project-page.ts";

/**
 * *Project settings*, `/p/WEB/settings` (PRJ-01, PRJ-03): the name and short name of a project and
 * *Move*, for the people who may change them, normally its project leads; the tab *Who has which
 * role* is next to them. A person who may do none of these finds nothing here, like at an address
 * that does not exist.
 */
export function ProjectSettingsPage({ address }: { readonly address: string }) {
  return (
    <ProjectGate address={address} kind={projectPage.settings}>
      {(project) => <ProjectSettings project={project} address={address} />}
    </ProjectGate>
  );
}
