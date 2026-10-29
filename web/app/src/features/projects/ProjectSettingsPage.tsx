import { mayOpenProjectPage, projectPages } from "../../shared/project-pages.ts";
import { ProjectGate } from "./ProjectGate.tsx";
import { OtherTab } from "./OtherTab.tsx";
import { ProjectSettings } from "./ProjectSettings.tsx";

/**
 * *Project settings*, `/p/WEB/settings` (PRJ-01, PRJ-03): the name and short name of a project and
 * *Move*, for the people who may change them, normally its project leads; the tab *Who has which
 * role* and what roles see there are tabs next to them. A person who may open none of them finds
 * nothing here, like at an address that does not exist.
 */
export function ProjectSettingsPage({ address }: { readonly address: string }) {
  return (
    <ProjectGate address={address} page={projectPages.settings}>
      {(project) =>
        mayOpenProjectPage(projectPages.settings, project) ? (
          <ProjectSettings project={project} address={address} />
        ) : (
          <OtherTab project={project} />
        )
      }
    </ProjectGate>
  );
}
