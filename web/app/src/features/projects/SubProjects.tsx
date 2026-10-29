import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Card,
  Link,
  RowChevron,
  partOf,
  part,
  ShowMoreOf,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { projectAddress } from "../../shared/paths.ts";
import { CreateProjectForm } from "./CreateProjectForm.tsx";
import { linesOf, useProjectList, type OneProject } from "./use-projects.ts";

/** The sub-projects the person sees, a page at a time. */
function SubProjectList({ project }: { readonly project: string }) {
  const list = useProjectList({ inside: project });
  return (
    <>
      <ul {...partOf(part.list, { variants: ["snug"] })} aria-label={t("projects.sub-projects")}>
        {linesOf(list.data?.pages).map((line) => (
          <li key={line.id} {...partOf(part.item, { variants: ["spread", "wrap"] })}>
            <span {...partOf(part.rowMain)}>
              <Link row href={projectAddress(line.shortName)}>
                {line.name}
              </Link>
              <span {...partOf(part.muted)}>{line.shortName}</span>
            </span>
            <RowChevron />
          </li>
        ))}
      </ul>
      {<ShowMoreOf label={t("projects.sub-projects-more")} list={list} />}
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
  const mayCreate = project.permissions.includes("create-sub-projects");
  const empty = list.isSuccess && linesOf(list.data.pages).length === 0;
  return (
    <Card heading={t("projects.sub-projects")}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {empty ? <p {...partOf(part.text)}>{t("projects.sub-projects-empty")}</p> : null}
      {empty ? null : <SubProjectList project={id} />}
      {empty && !mayCreate ? <p {...partOf(part.muted)}>{t("projects.sub-projects-ask")}</p> : null}
      {mayCreate && !making ? (
        <div {...partOf(part.row)}>
          <Button
            look={empty ? buttonLook.primary : buttonLook.plain}
            onPress={() => {
              setMaking(true);
            }}
          >
            {t("projects.create-sub-project")}
          </Button>
        </div>
      ) : null}
      {mayCreate && making ? <CreateProjectForm inside={id} /> : null}
    </Card>
  );
}
