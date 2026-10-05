import { t } from "@bugaboo/messages";
import { Alert, Button, Card, Link } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { projectAddress, projectSettingsAddress } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ListEditor, type ListRow } from "../shell/index.ts";
import { ChangeProjectForm } from "./ChangeProjectForm.tsx";
import { MakeProjectForm } from "./MakeProjectForm.tsx";
import styles from "./Projects.module.css";
import { linesOf, useProject, useProjectList, type ProjectLine } from "./use-projects.ts";

/** The editor of one project in the side panel: its name and short name, and the way to its pages. */
function ProjectEditor({ shortName }: { readonly shortName: string }) {
  const project = useProject(shortName);
  if (project.isError) {
    return <Alert danger>{messageOf(project.error)}</Alert>;
  }
  if (project.isPending) {
    return <p className={styles.muted}>{t("common.loading")}</p>;
  }
  const changer = project.data.permissions.includes("change-project-settings");
  return (
    <>
      {changer ? <ChangeProjectForm key={project.data.project.id} project={project.data} /> : null}
      <div className={styles.row}>
        <Link href={projectAddress(shortName)}>{t("projects.open")}</Link>
        <Link href={projectSettingsAddress(shortName)}>{t("projects.settings-link")}</Link>
      </div>
    </>
  );
}

/** Where a project sits and its short name, after its name in the list. */
function Details({ project }: { readonly project: ProjectLine }) {
  const place =
    project.parent === null
      ? t("projects.at-the-top")
      : t("projects.inside", { name: project.parent.name });
  return (
    <span className={styles.details}>
      <span>{project.shortName}</span>
      <span className={styles.muted}>{place}</span>
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
    <div className={styles.stack}>
      <Card heading={t("projects.title")} level={1}>
        <p className={styles.text}>{t("projects.intro")}</p>
        {list.isError ? <Alert danger>{messageOf(list.error)}</Alert> : null}
        {list.isSuccess && rows.length === 0 ? (
          <p className={styles.muted}>{t("projects.none-yet")}</p>
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
      <Card heading={t("projects.make-title")}>
        <p className={styles.text}>{t("projects.make-intro")}</p>
        <MakeProjectForm />
      </Card>
    </div>
  );
}
