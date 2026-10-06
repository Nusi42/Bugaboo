import type { Schema } from "@bugaboo/api-client";

import { ProjectGate, projectPage } from "../projects/index.ts";
import {
  ProjectAccessCard,
  ProjectCheckAccessPage,
  ProjectFieldPermissionsPage,
  ProjectPeopleVisibilityPage,
  type AccessProject,
} from "./project-pages.tsx";

/** One project as the pages of *What roles see here* need it. */
type Detail = Schema<"ProjectDetailBody">;

function accessProject(detail: Detail): AccessProject {
  return {
    id: detail.project.id,
    shortName: detail.project.short_name,
    name: detail.project.name,
    permissions: detail.permissions,
  };
}

interface AddressProps {
  /** The address as it stands in the address bar: a short name or an ID. */
  readonly address: string;
}

/** `/p/WEB/settings/fields`: a project that the person may not see is *Not found*, as everywhere. */
export function ProjectFieldPermissionsScreen({ address }: AddressProps) {
  return (
    <ProjectGate address={address} kind={projectPage.settings}>
      {(project) => <ProjectFieldPermissionsPage project={accessProject(project)} />}
    </ProjectGate>
  );
}

/** `/p/WEB/settings/people`. */
export function ProjectPeopleVisibilityScreen({ address }: AddressProps) {
  return (
    <ProjectGate address={address} kind={projectPage.settings}>
      {(project) => <ProjectPeopleVisibilityPage project={accessProject(project)} />}
    </ProjectGate>
  );
}

/** `/p/WEB/settings/check-access`. */
export function ProjectCheckAccessScreen({ address }: AddressProps) {
  return (
    <ProjectGate address={address} kind={projectPage.settings}>
      {(project) => <ProjectCheckAccessPage project={accessProject(project)} />}
    </ProjectGate>
  );
}

/** The card of *Project settings* that leads to these pages. */
export function ProjectAccessCardScreen({ project }: { readonly project: Detail }) {
  return <ProjectAccessCard project={accessProject(project)} />;
}
