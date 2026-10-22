import { t } from "@bugaboo/messages";

import type { LayoutsOpen } from "../../shared/paths.ts";
import { projectPages } from "../../shared/project-pages.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ProjectSettingsScreen } from "../projects/index.ts";
import { LayoutsCard } from "./LayoutsCard.tsx";

export interface LayoutsScreenProps {
  readonly open: LayoutsOpen;
  /** Opens another form or issue type: the address changes. */
  readonly onOpen: (shortName: string | undefined, open: LayoutsOpen) => void;
}

export interface ProjectLayoutsScreenProps extends LayoutsScreenProps {
  /** The address as it stands in the address bar: a short name or an ID. */
  readonly address: string;
}

/** `/p/WEB/settings/layouts`: the layouts of a project, below the tabs of its settings. */
export function ProjectLayoutsScreen({ address, open, onOpen }: ProjectLayoutsScreenProps) {
  return (
    <ProjectSettingsScreen address={address} page={projectPages.layouts}>
      {(project) => (
        <LayoutsCard
          project={project.project.short_name}
          heading={t("layouts.project-title", { project: project.project.name })}
          open={open}
          onOpen={(next) => {
            onOpen(project.project.short_name, next);
          }}
        />
      )}
    </ProjectSettingsScreen>
  );
}

/** `/settings/layouts`: the layouts of the whole organization, which projects use until they have their own. */
export function OrganizationLayoutsScreen({ open, onOpen }: LayoutsScreenProps) {
  usePageTitle(t("layouts.nav"));
  return (
    <LayoutsCard
      project={undefined}
      heading={t("layouts.organization-title")}
      open={open}
      onOpen={(next) => {
        onOpen(undefined, next);
      }}
    />
  );
}
