import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";
import type { ReactNode } from "react";

import type { PermissionKey } from "../../shared/permissions.ts";
import { ProjectTabs, type OneProject } from "../projects/index.ts";
import { NotFoundPage } from "../shell/index.ts";
import styles from "./Access.module.css";
import { CheckAccessView } from "./CheckAccessView.tsx";
import { FieldPermissionsTable, PeopleVisibilityTable } from "./tables.tsx";

/** The permission that opens the field permissions and *Who sees people*. */
const DECIDE = "decide-what-roles-see";
/** The permission that opens *Check access*. */
const CHECK = "check-access";

interface ProjectFrameProps {
  readonly project: OneProject;
  /** The permission the page needs in the project; a person without it finds nothing, as at an address that is not there. */
  readonly needs: PermissionKey;
  readonly heading: string;
  readonly intro: string;
  readonly children: ReactNode;
}

/** What every page of *What roles see here* has: the tabs of the settings, the heading and the introduction. */
function ProjectFrame({ project, needs, heading, intro, children }: ProjectFrameProps) {
  if (!project.permissions.includes(needs)) {
    return <NotFoundPage />;
  }
  return (
    <div className={styles.stack}>
      <ProjectTabs project={project} />
      <Card heading={heading} level={1}>
        <div className={styles.stack}>
          <p className={styles.intro}>{intro}</p>
          {children}
        </div>
      </Card>
    </div>
  );
}

/** The field permissions of a project, with the levels that come from above marked (SEC-04, PRJ-02). */
export function ProjectFieldPermissionsPage({ project }: { readonly project: OneProject }) {
  return (
    <ProjectFrame
      project={project}
      needs={DECIDE}
      heading={t("field-permissions.project-title", { project: project.project.name })}
      intro={t("field-permissions.intro-project", { project: project.project.name })}
    >
      <FieldPermissionsTable project={project.project.id} />
    </ProjectFrame>
  );
}

/** *Who sees people* in a project, with the levels that come from above marked (PRIV-12, PRJ-02). */
export function ProjectPeopleVisibilityPage({ project }: { readonly project: OneProject }) {
  return (
    <ProjectFrame
      project={project}
      needs={DECIDE}
      heading={t("people-visibility.project-title", { project: project.project.name })}
      intro={t("people-visibility.intro-project", { project: project.project.name })}
    >
      <PeopleVisibilityTable project={project.project.id} />
    </ProjectFrame>
  );
}

/** *Check access* in a project (SEC-03). */
export function ProjectCheckAccessPage({ project }: { readonly project: OneProject }) {
  return (
    <ProjectFrame
      project={project}
      needs={CHECK}
      heading={t("check-access.project-title", { project: project.project.name })}
      intro={t("check-access.intro")}
    >
      <CheckAccessView project={project.project.id} />
    </ProjectFrame>
  );
}
