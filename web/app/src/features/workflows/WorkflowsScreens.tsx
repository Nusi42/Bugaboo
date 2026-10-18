import { t } from "@bugaboo/messages";

import type { WorkflowsOpen } from "../../shared/paths.ts";
import { projectPages } from "../../shared/project-pages.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ProjectSettingsScreen } from "../projects/index.ts";
import { WorkflowsCard } from "./WorkflowsCard.tsx";

export interface WorkflowsScreenProps {
  readonly open: WorkflowsOpen;
  /** Opens another issue type: the address changes. */
  readonly onOpen: (shortName: string | undefined, open: WorkflowsOpen) => void;
}

export interface ProjectWorkflowsScreenProps extends WorkflowsScreenProps {
  /** The address as it stands in the address bar: a short name or an ID. */
  readonly address: string;
}

/** `/p/WEB/settings/workflows`: the workflows of a project, below the tabs of its settings. */
export function ProjectWorkflowsScreen({ address, open, onOpen }: ProjectWorkflowsScreenProps) {
  return (
    <ProjectSettingsScreen address={address} page={projectPages.workflows}>
      {(project) => (
        <WorkflowsCard
          project={project.project.short_name}
          heading={t("workflows.project-title", { project: project.project.name })}
          open={open}
          onOpen={(next) => {
            onOpen(project.project.short_name, next);
          }}
        />
      )}
    </ProjectSettingsScreen>
  );
}

/** `/settings/workflows`: the workflows of the whole organization, which projects use until they have their own. */
export function OrganizationWorkflowsScreen({ open, onOpen }: WorkflowsScreenProps) {
  usePageTitle(t("workflows.nav"));
  return (
    <WorkflowsCard
      project={undefined}
      heading={t("workflows.organization-title")}
      open={open}
      onOpen={(next) => {
        onOpen(undefined, next);
      }}
    />
  );
}
