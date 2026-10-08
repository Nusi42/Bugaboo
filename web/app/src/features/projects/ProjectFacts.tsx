import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import { projectPageAddress, projectPages } from "../../shared/project-pages.ts";
import { tabsOf } from "./settings-tabs.ts";
import type { OneProject } from "./use-projects.ts";

/** The facts of a project: its short name, and where it sits when the person sees that. */
function Facts({ project }: { readonly project: OneProject }) {
  const parent = project.trail.at(-1);
  return (
    <dl {...partOf(part.projectsFacts)}>
      <div {...partOf(part.projectsFact)}>
        <dt {...partOf(part.projectsTerm)}>{t("projects.overview-short-name")}</dt>
        <dd {...partOf(part.projectsDefinition)}>{project.project.short_name}</dd>
      </div>
      <div {...partOf(part.projectsFact)}>
        <dt {...partOf(part.projectsTerm)}>{t("projects.overview-inside")}</dt>
        <dd {...partOf(part.projectsDefinition)}>
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
    <div {...partOf(part.row)}>
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
