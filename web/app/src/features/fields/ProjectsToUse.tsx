import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { PickerOption, pickerLook, SearchPicker } from "@bugaboo/ui";
import { keepPreviousData } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";

/** A project a field can be limited to. */
export interface ProjectChoice {
  readonly id: string;
  readonly name: string;
  readonly shortName: string;
}

export interface ProjectsToUseProps {
  /** The IDs of the projects that are in the list already: they are not offered again. */
  readonly picked: readonly string[];
  /** A project was picked: it is added to the projects the field is used in. */
  readonly onPick: (project: ProjectChoice) => void;
}

/** The projects the person sees, found by words, a few at a time. */
function useProjectChoices(search: string) {
  return usePaged(
    {
      queryKey: ["fields", "projects", search],
      read: (after) =>
        api.client.GET("/api/v1/projects", {
          params: {
            query: {
              search,
              fields: fieldList<Schema<"ProjectItem">>(["id", "short_name", "name"]),
              limit: pageSize.short,
              ...after,
            },
          },
        }),
    },
    { placeholderData: keepPreviousData },
  );
}

/** *Where it's used*: the projects a field can be limited to, found by words; picking one adds it. */
export function ProjectsToUse({ picked, onPick }: ProjectsToUseProps) {
  const { words, group, frame } = usePickerFrame();
  const found = useProjectChoices(words);
  const projects = linesFrom(found.data?.pages, (item): ProjectChoice | undefined => {
    const { id, short_name: shortName, name } = item;
    return id === undefined || id === null || shortName === undefined || shortName === null
      ? undefined
      : { id, shortName, name: name ?? shortName };
  }).filter((project) => !picked.includes(project.id));
  return (
    <SearchPicker
      look={pickerLook.boxed}
      legend={t("fields.projects-legend")}
      searchLabel={t("fields.projects-search")}
      listLabel={t("fields.projects-list")}
      {...frame}
      {...pickerStatus(found, projects.length, {
        nothing: t("fields.projects-nothing"),
        more: t("fields.projects-more"),
      })}
    >
      {projects.map((project) => (
        <PickerOption
          key={project.id}
          group={group}
          checked={false}
          label={project.name}
          hint={project.shortName}
          onPick={() => {
            onPick(project);
          }}
        />
      ))}
    </SearchPicker>
  );
}
