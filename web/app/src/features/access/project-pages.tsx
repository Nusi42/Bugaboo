import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { mayOpenProjectPage, projectPages, type ProjectPage } from "../../shared/project-pages.ts";
import { ProjectSettingsFrame, type OneProject } from "../projects/index.ts";
import { NotFoundPage } from "../shell/index.ts";
import { CheckAccessView } from "./CheckAccessView.tsx";
import { FieldPermissionsTable, PeopleVisibilityTable } from "./tables.tsx";

interface ProjectFrameProps {
  readonly project: OneProject;
  /** The page; a person who may not open it finds nothing, as at an address that is not there. */
  readonly page: ProjectPage;
  readonly heading: string;
  readonly intro: string;
  readonly children: ReactNode;
}

/** What every page of *What roles see here* has: the tabs of the settings, the heading and the introduction. */
function ProjectFrame({ project, page, heading, intro, children }: ProjectFrameProps) {
  if (!mayOpenProjectPage(page, project)) {
    return <NotFoundPage />;
  }
  return (
    <ProjectSettingsFrame project={project}>
      <Card heading={heading} level={1}>
        <div {...partOf(part.stack)}>
          <p {...partOf(part.muted)}>{intro}</p>
          {children}
        </div>
      </Card>
    </ProjectSettingsFrame>
  );
}

/** The field permissions of a project, with the levels that come from above marked (SEC-04, PRJ-02). */
export function ProjectFieldPermissionsPage({ project }: { readonly project: OneProject }) {
  return (
    <ProjectFrame
      project={project}
      page={projectPages.fields}
      heading={t("field-permissions.project-title", {
        project: project.project.name,
      })}
      intro={t("field-permissions.intro-project", {
        project: project.project.name,
      })}
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
      page={projectPages.people}
      heading={t("people-visibility.project-title", {
        project: project.project.name,
      })}
      intro={t("people-visibility.intro-project", {
        project: project.project.name,
      })}
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
      page={projectPages.check}
      heading={t("check-access.project-title", {
        project: project.project.name,
      })}
      intro={t("check-access.intro")}
    >
      <CheckAccessView project={project.project.id} />
    </ProjectFrame>
  );
}
