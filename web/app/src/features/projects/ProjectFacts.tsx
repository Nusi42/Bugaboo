import { t } from "@bugaboo/messages";
import { Link } from "@bugaboo/ui";

import { projectPageAddress, projectPages } from "../../shared/project-pages.ts";
import styles from "./Projects.module.css";
import { tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/** The facts of a project: its short name, and where it sits when the person sees that. */
function Facts({ project }: { readonly project: OneProject }) {
  const parent = project.trail.at(-1);
  return (
    <dl className={styles.facts}>
      <div className={styles.fact}>
        <dt className={styles.term}>{t("projects.overview-short-name")}</dt>
        <dd className={styles.definition}>{project.project.short_name}</dd>
      </div>
      <div className={styles.fact}>
        <dt className={styles.term}>{t("projects.overview-inside")}</dt>
        <dd className={styles.definition}>
          {parent === undefined ? t("projects.overview-top") : parent.name}
        </dd>
      </div>
    </dl>
  );
}

/** The ways from the overview of a project to its settings: the first tab, and *Who has which role*. */
function ProjectLinks({ project }: { readonly project: OneProject }) {
  const tabs = tabsOf(project);
  const [first] = tabs;
  const shortName = project.project.short_name;
  return (
    <div className={styles.row}>
      {first === undefined ? null : (
        <Link href={projectPageAddress(first, shortName)}>{t("projects.settings-link")}</Link>
      )}
      {first !== projectPages.roles && tabs.includes(projectPages.roles) ? (
        <Link href={projectPageAddress(projectPages.roles, shortName)}>
          {t("roles.project-link")}
        </Link>
      ) : null}
    </div>
  );
}

/** What the overview says about a project: its facts, and the ways to its settings. */
export function ProjectFacts({ project }: { readonly project: OneProject }) {
  return (
    <>
      <Facts project={project} />
      <ProjectLinks project={project} />
    </>
  );
}
