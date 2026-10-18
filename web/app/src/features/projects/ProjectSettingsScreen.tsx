import { partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import type { ProjectPage } from "../../shared/project-pages.ts";
import { ProjectGate } from "./ProjectGate.tsx";
import { ProjectTabs } from "./ProjectTabs.tsx";
import type { OneProject } from "./use-projects.ts";

export interface ProjectSettingsScreenProps {
  /** The address as it stands in the address bar: a short name or an ID. */
  readonly address: string;
  readonly page: ProjectPage;
  /** The cards of the page, once the project is there. */
  readonly children: (project: OneProject) => ReactNode;
}

/**
 * A page of a project's settings: the gate that makes a project the person may not see *Not found*,
 * the tabs of the settings, and the page below them. Written once for the layouts, the workflows,
 * the issue types and the labels of a project.
 */
export function ProjectSettingsScreen({ address, page, children }: ProjectSettingsScreenProps) {
  return (
    <ProjectGate address={address} page={page}>
      {(project) => (
        <div {...partOf(part.stack)}>
          <ProjectTabs project={project} />
          {children(project)}
        </div>
      )}
    </ProjectGate>
  );
}
