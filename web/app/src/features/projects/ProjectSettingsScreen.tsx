import type { ReactNode } from "react";

import type { ProjectPage } from "../../shared/project-pages.ts";
import { ProjectGate } from "./ProjectGate.tsx";
import { ProjectSettingsFrame } from "./ProjectSettingsFrame.tsx";
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
 * the list of the settings pages, and the page. Written once for the layouts, the workflows, the
 * issue types and the labels of a project.
 */
export function ProjectSettingsScreen({ address, page, children }: ProjectSettingsScreenProps) {
  return (
    <ProjectGate address={address} page={page}>
      {(project) => (
        <ProjectSettingsFrame project={project}>{children(project)}</ProjectSettingsFrame>
      )}
    </ProjectGate>
  );
}
