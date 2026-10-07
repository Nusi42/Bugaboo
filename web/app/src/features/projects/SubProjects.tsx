import { t } from "@bugaboo/messages";
import { Alert, Button, Card, Link } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { projectAddress } from "../../shared/paths.ts";
import { MakeProjectForm } from "./MakeProjectForm.tsx";
import styles from "./Projects.module.css";
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
export function SubProjects({ project }: { readonly project: OneProject }) {
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
