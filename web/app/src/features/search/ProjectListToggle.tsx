import { t } from "@bugaboo/messages";
import { Button, buttonLook } from "@bugaboo/ui";

import { mayChangeProject } from "../../shared/project-pages.ts";
import { useProject } from "../projects/index.ts";
import { useProjectListChoice, useSetProjectList, type SavedSearch } from "./saved-searches.ts";

export interface ProjectListToggleProps {
  readonly search: SavedSearch;
  /** The short name of a project the saved search is shared with. */
  readonly project: string;
}

/**
 * *Use as the project's list* (ADR-0034 decision 6): the button is there only for a person who
 * may change the settings of the project, and only while the search is shared with the project.
 */
export function ProjectListToggle({ search, project }: ProjectListToggleProps) {
  const detail = useProject(project).data;
  const chosen = useProjectListChoice(project).data;
  const set = useSetProjectList(project);
  if (detail === undefined || !mayChangeProject(detail)) {
    return null;
  }
  const isChosen = chosen?.search?.id === search.id;
  return (
    <Button
      look={buttonLook.quiet}
      isPending={set.isPending}
      onPress={() => {
        set.mutate(isChosen ? null : search.id);
      }}
    >
      {isChosen
        ? t("search.project-list-stop", { project })
        : t("search.project-list-use", { project })}
    </Button>
  );
}
