import { t } from "@bugaboo/messages";
import { Alert } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { codeOf, messageOf } from "../../shared/errors.ts";
import type { ProjectPage } from "../../shared/project-pages.ts";
import { NotFoundPage } from "../shell/index.ts";
import styles from "./Projects.module.css";
import { useProjectPage } from "./use-project-page.ts";
import type { OneProject } from "./use-projects.ts";

export interface ProjectGateProps {
  /** The address as it stands in the address bar: a short name (also an old one) or an ID. */
  readonly address: string;
  readonly page: ProjectPage;
  /** The page, once the project is there. */
  readonly children: (project: OneProject) => ReactNode;
}

/**
 * The part of every page of a project that comes before the page: a project the person may not
 * see is *Not found*, exactly like one that does not exist, and while it loads nothing is claimed.
 */
export function ProjectGate({ address, page, children }: ProjectGateProps) {
  const project = useProjectPage(address, page);
  if (project.isError) {
    return codeOf(project.error) === "resource.not_found" ? (
      <NotFoundPage />
    ) : (
      <Alert danger>{messageOf(project.error)}</Alert>
    );
  }
  if (project.data === undefined) {
    return <p className={styles.muted}>{t("common.loading")}</p>;
  }
  return children(project.data);
}
