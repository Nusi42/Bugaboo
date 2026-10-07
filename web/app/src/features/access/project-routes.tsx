import { ProjectGate, projectPage } from "../projects/index.ts";
import {
  ProjectCheckAccessPage,
  ProjectFieldPermissionsPage,
  ProjectPeopleVisibilityPage,
} from "./project-pages.tsx";

interface AddressProps {
  /** The address as it stands in the address bar: a short name or an ID. */
  readonly address: string;
}

/** `/p/WEB/settings/fields`: a project that the person may not see is *Not found*, as everywhere. */
export function ProjectFieldPermissionsScreen({ address }: AddressProps) {
  return (
    <ProjectGate address={address} kind={projectPage.fields}>
      {(project) => <ProjectFieldPermissionsPage project={project} />}
    </ProjectGate>
  );
}

/** `/p/WEB/settings/people`. */
export function ProjectPeopleVisibilityScreen({ address }: AddressProps) {
  return (
    <ProjectGate address={address} kind={projectPage.people}>
      {(project) => <ProjectPeopleVisibilityPage project={project} />}
    </ProjectGate>
  );
}

/** `/p/WEB/settings/check-access`. */
export function ProjectCheckAccessScreen({ address }: AddressProps) {
  return (
    <ProjectGate address={address} kind={projectPage.check}>
      {(project) => <ProjectCheckAccessPage project={project} />}
    </ProjectGate>
  );
}
