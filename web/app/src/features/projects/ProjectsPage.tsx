import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Card, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ListEditor, type ListRow } from "../shell/index.ts";
import { MakeProjectForm } from "./MakeProjectForm.tsx";
import { ProjectEditor } from "./ProjectEditor.tsx";
import { linesOf, useProjectList, type ProjectLine } from "./use-projects.ts";

/** Where a project sits and its short name, after its name in the list. */
function Details({ project }: { readonly project: ProjectLine }) {
  const place =
    project.parent === null
      ? t("projects.at-the-top")
      : t("projects.inside", { name: project.parent.name });
  return (
    <span {...partOf(part.projectsDetails)}>
      <span>{project.shortName}</span>
      <span {...partOf(part.muted)}>{place}</span>
    </span>
  );
}

export interface ProjectsPageProps {
  /** The short name of the project that is open in the side panel, from the address. */
  readonly open: string | undefined;
  readonly onOpen: (shortName: string) => void;
  readonly onClose: () => void;
}

/**
 * *Organization settings › Projects* (PRJ-01): the projects the person sees as a searchable list,
 * where one opens next to it to change its name; and the form that makes a project at the top
 * level. Sub-projects are made inside their project.
 */
export function ProjectsPage({ open, onOpen, onClose }: ProjectsPageProps) {
  usePageTitle(t("projects.title"));
  const list = useProjectList({ search: "" });
  const rows: readonly ListRow[] = linesOf(list.data?.pages).map((project) => ({
    key: project.shortName,
    name: project.name,
    details: <Details project={project} />,
    editor: <ProjectEditor shortName={project.shortName} />,
  }));
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("projects.title")} level={1}>
        <p {...partOf(part.text)}>{t("projects.intro")}</p>
        {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
        {list.isSuccess && rows.length === 0 ? (
          <p {...partOf(part.muted)}>{t("projects.none-yet")}</p>
        ) : (
          <ListEditor
            rows={rows}
            searchLabel={t("projects.search")}
            nothing={t("projects.nothing")}
            openKey={open}
            onOpen={onOpen}
            onClose={onClose}
          />
        )}
        {list.hasNextPage ? (
          <Button
            isPending={list.isFetchingNextPage}
            onPress={() => {
              void list.fetchNextPage();
            }}
          >
            {t("projects.more")}
          </Button>
        ) : null}
      </Card>
      <Card heading={t("projects.create-title")}>
        <p {...partOf(part.text)}>{t("projects.create-intro")}</p>
        <MakeProjectForm />
      </Card>
    </div>
  );
}
