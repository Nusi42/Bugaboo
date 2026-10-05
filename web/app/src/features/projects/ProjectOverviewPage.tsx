import { t } from "@bugaboo/messages";
import { Alert, Button, Card, Link } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { projectAddress, projectSettingsAddress } from "../../shared/paths.ts";
import { MakeProjectForm } from "./MakeProjectForm.tsx";
import { ProjectGate } from "./ProjectGate.tsx";
import styles from "./Projects.module.css";
import { projectPage } from "./use-project-page.ts";
import { linesOf, useProjectList, type OneProject } from "./use-projects.ts";

/** The sub-projects the person sees, a page at a time. */
function SubProjectList({ project }: { readonly project: string }) {
  const list = useProjectList({ inside: project });
  return (
    <>
      <ul className={styles.items} aria-label={t("projects.sub-projects")}>
        {linesOf(list.data?.pages).map((line) => (
          <li key={line.id} className={styles.item}>
            <Link href={projectAddress(line.shortName)}>{line.name}</Link>
            <span className={styles.muted}>{line.shortName}</span>
          </li>
        ))}
      </ul>
      {list.hasNextPage ? (
        <Button
          isPending={list.isFetchingNextPage}
          onPress={() => {
            void list.fetchNextPage();
          }}
        >
          {t("projects.sub-projects-more")}
        </Button>
      ) : null}
    </>
  );
}

/**
 * The sub-projects of the project, and what to do next when there are none: make one, for those
 * who may; the others are told whom to ask.
 */
function SubProjects({ project }: { readonly project: OneProject }) {
  const { id } = project.project;
  const list = useProjectList({ inside: id });
  const [making, setMaking] = useState(false);
  const mayMake = project.permissions.includes("make-sub-projects");
  const empty = list.isSuccess && linesOf(list.data.pages).length === 0;
  return (
    <Card heading={t("projects.sub-projects")}>
      {list.isError ? <Alert danger>{messageOf(list.error)}</Alert> : null}
      {empty ? <p className={styles.text}>{t("projects.sub-projects-empty")}</p> : null}
      {empty ? null : <SubProjectList project={id} />}
      {empty && !mayMake ? <p className={styles.muted}>{t("projects.sub-projects-ask")}</p> : null}
      {mayMake && !making ? (
        <div className={styles.row}>
          <Button
            primary={empty}
            onPress={() => {
              setMaking(true);
            }}
          >
            {t("projects.make-sub-project")}
          </Button>
        </div>
      ) : null}
      {mayMake && making ? <MakeProjectForm inside={id} /> : null}
    </Card>
  );
}

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

/** The page itself, once the project is known. */
function Overview({ project }: { readonly project: OneProject }) {
  const mayChange = project.permissions.includes("change-project-settings") || project.may_move;
  return (
    <div className={styles.stack}>
      <Card heading={project.project.name} level={1}>
        <Facts project={project} />
        {mayChange ? (
          <div className={styles.row}>
            <Link href={projectSettingsAddress(project.project.short_name)}>
              {t("projects.settings-link")}
            </Link>
          </div>
        ) : null}
      </Card>
      <SubProjects project={project} />
    </div>
  );
}

/**
 * The overview of a project, `/p/WEB` (UX §2.1): its name, short name and where it is, its
 * sub-projects, and the way to its settings for those who may change them. An old short name opens
 * the project and the address is replaced by the current one. A project the person may not see is
 * not found, like one that does not exist.
 */
export function ProjectOverviewPage({ address }: { readonly address: string }) {
  return (
    <ProjectGate address={address} kind={projectPage.overview}>
      {(project) => <Overview project={project} />}
    </ProjectGate>
  );
}
