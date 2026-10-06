import { t } from "@bugaboo/messages";
import { Card, Link } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { paths, projectPageAddress, projectSettingsAddress } from "../../shared/paths.ts";
import type { PermissionKey } from "../../shared/permissions.ts";
import { NotFoundPage } from "../shell/index.ts";
import styles from "./Access.module.css";
import { CheckAccessView } from "./CheckAccessView.tsx";
import { FieldPermissionsTable, PeopleVisibilityTable } from "./tables.tsx";

/** The permission that opens the field permissions and *Who sees people*. */
const DECIDE = "decide-what-roles-see";
/** The permission that opens *Check access*. */
const CHECK = "check-access";

/** The project a page of *What roles see here* is about: what the page needs to know of it. */
export interface AccessProject {
  readonly id: string;
  readonly shortName: string;
  readonly name: string;
  /** What the person may do in the project. */
  readonly permissions: readonly PermissionKey[];
}

/** The links to the pages of *What roles see here*, for those the person may open. */
function AccessLinks({ project }: { readonly project: AccessProject }) {
  const decide = project.permissions.includes(DECIDE);
  const check = project.permissions.includes(CHECK);
  return (
    <nav aria-label={t("access.project-links-name")}>
      <ul className={styles.nav}>
        {decide ? (
          <>
            <li>
              <Link href={projectPageAddress(paths.projectFields, project.shortName)}>
                {t("nav.field-permissions")}
              </Link>
            </li>
            <li>
              <Link href={projectPageAddress(paths.projectPeople, project.shortName)}>
                {t("nav.people-visibility")}
              </Link>
            </li>
          </>
        ) : null}
        {check ? (
          <li>
            <Link href={projectPageAddress(paths.projectCheckAccess, project.shortName)}>
              {t("nav.check-access")}
            </Link>
          </li>
        ) : null}
      </ul>
    </nav>
  );
}

/**
 * The card on *Project settings* that leads to what roles see in the project; nothing for a person
 * who may open none of its pages.
 */
export function ProjectAccessCard({ project }: { readonly project: AccessProject }) {
  if (!project.permissions.includes(DECIDE) && !project.permissions.includes(CHECK)) {
    return null;
  }
  return (
    <Card heading={t("access.project-links-title")}>
      <p className={styles.intro}>{t("access.project-links-intro")}</p>
      <AccessLinks project={project} />
    </Card>
  );
}

interface ProjectFrameProps {
  readonly project: AccessProject;
  /** The permission the page needs in the project; a person without it finds nothing, as at an address that is not there. */
  readonly needs: PermissionKey;
  readonly heading: string;
  readonly intro: string;
  readonly children: ReactNode;
}

/** What every page of *What roles see here* has: the links to its sisters, *Back*, the heading and the introduction. */
function ProjectFrame({ project, needs, heading, intro, children }: ProjectFrameProps) {
  if (!project.permissions.includes(needs)) {
    return <NotFoundPage />;
  }
  return (
    <Card heading={heading} level={1}>
      <div className={styles.stack}>
        <AccessLinks project={project} />
        <Link href={projectSettingsAddress(project.shortName)}>{t("access.back-to-settings")}</Link>
        <p className={styles.intro}>{intro}</p>
        {children}
      </div>
    </Card>
  );
}

/** The field permissions of a project, with the levels that come from above marked (SEC-04, PRJ-02). */
export function ProjectFieldPermissionsPage({ project }: { readonly project: AccessProject }) {
  return (
    <ProjectFrame
      project={project}
      needs={DECIDE}
      heading={t("field-permissions.project-title", { project: project.name })}
      intro={t("field-permissions.intro-project", { project: project.name })}
    >
      <FieldPermissionsTable project={project.id} />
    </ProjectFrame>
  );
}

/** *Who sees people* in a project, with the levels that come from above marked (PRIV-12, PRJ-02). */
export function ProjectPeopleVisibilityPage({ project }: { readonly project: AccessProject }) {
  return (
    <ProjectFrame
      project={project}
      needs={DECIDE}
      heading={t("people-visibility.project-title", { project: project.name })}
      intro={t("people-visibility.intro-project", { project: project.name })}
    >
      <PeopleVisibilityTable project={project.id} />
    </ProjectFrame>
  );
}

/** *Check access* in a project (SEC-03). */
export function ProjectCheckAccessPage({ project }: { readonly project: AccessProject }) {
  return (
    <ProjectFrame
      project={project}
      needs={CHECK}
      heading={t("check-access.project-title", { project: project.name })}
      intro={t("check-access.intro")}
    >
      <CheckAccessView project={project.id} />
    </ProjectFrame>
  );
}
