import { t } from "@bugaboo/messages";
import { Button, buttonLook, Switch, partOf, part } from "@bugaboo/ui";

import { ProjectsToUse, type ProjectChoice } from "../fields/index.ts";

/** The words of the picker of the projects a tag is limited to. */
function projectWords() {
  return {
    legend: t("labels.tag-projects-list"),
    search: t("fields.projects-search"),
    list: t("fields.projects-list"),
    nothing: t("fields.projects-nothing"),
    more: t("fields.projects-more"),
  };
}

/** The projects a tag is limited to, each with the button that takes it off, and the picker. */
function TagProjects(props: {
  readonly projects: readonly ProjectChoice[];
  readonly setProjects: (projects: readonly ProjectChoice[]) => void;
}) {
  const { projects, setProjects } = props;
  return (
    <div {...partOf(part.stack)}>
      <ul {...partOf(part.list)} aria-label={t("labels.tag-projects-list")}>
        {projects.map((project) => (
          <li key={project.id} {...partOf(part.item, { variants: ["spread", "slim"] })}>
            <span>{project.name}</span>
            <Button
              look={buttonLook.quiet}
              aria-label={t("labels.take-project-off", { name: project.name })}
              onPress={() => {
                setProjects(projects.filter((each) => each.id !== project.id));
              }}
            >
              {t("labels.take-off-short")}
            </Button>
          </li>
        ))}
      </ul>
      <ProjectsToUse
        picked={projects.map((each) => each.id)}
        words={projectWords()}
        onPick={(project) => {
          setProjects([...projects, project]);
        }}
      />
    </div>
  );
}

interface TagProjectsFieldProps {
  readonly everywhere: boolean;
  readonly onEverywhere: (everywhere: boolean) => void;
  readonly projects: readonly ProjectChoice[];
  readonly setProjects: (projects: readonly ProjectChoice[]) => void;
}

/** *Use in all projects*, or the projects the tag is limited to. */
export function TagProjectsField(props: TagProjectsFieldProps) {
  return (
    <>
      <p {...partOf(part.muted)}>{t("labels.tag-projects-help")}</p>
      <Switch
        label={t("labels.tag-everywhere-switch")}
        isSelected={props.everywhere}
        onChange={props.onEverywhere}
      />
      {props.everywhere ? null : (
        <TagProjects projects={props.projects} setProjects={props.setProjects} />
      )}
    </>
  );
}
