import { t } from "@bugaboo/messages";
import { PickerOption, SearchPicker } from "@bugaboo/ui";
import { useId, useState } from "react";

import { pageSize } from "../../shared/paged.ts";
import { pickerStatus } from "../../shared/picker-status.ts";
import { linesOf, useProjectList } from "./use-projects.ts";

export interface ProjectPickerProps {
  /** The project that moves: it, what is inside it, where it is and where the person may not move it to are not offered. */
  readonly moving: string;
  /** Whether the person may move projects to the top level. */
  readonly top: boolean;
  /** Where it goes: another project (an ID), the top level (`null`), or nothing picked yet. */
  readonly value: string | null | undefined;
  readonly onChange: (value: string | null) => void;
}

/**
 * Where a project goes (PRJ-03): the top level, or one of the projects the person finds by
 * searching. The server leaves out the places the person may not move it to; it decides again.
 */
export function ProjectPicker({ moving, top, value, onChange }: ProjectPickerProps) {
  const [query, setQuery] = useState("");
  const group = useId();
  const found = useProjectList({ search: query.trim(), movable: moving }, pageSize.short);
  const projects = linesOf(found.data?.pages);
  return (
    <SearchPicker
      legend={t("projects.picker-legend")}
      searchLabel={t("projects.picker-search")}
      clearLabel={t("search.clear")}
      listLabel={t("projects.picker-list")}
      query={query}
      onQuery={setQuery}
      {...pickerStatus(found, projects.length, {
        nothing: t("projects.picker-nothing"),
        more: t("projects.picker-more"),
      })}
    >
      {top ? (
        <PickerOption
          group={group}
          checked={value === null}
          label={t("projects.picker-top")}
          onPick={() => {
            onChange(null);
          }}
        />
      ) : null}
      {projects.map((project) => (
        <PickerOption
          key={project.id}
          group={group}
          checked={value === project.id}
          label={project.name}
          hint={project.shortName}
          onPick={() => {
            onChange(project.id);
          }}
        />
      ))}
    </SearchPicker>
  );
}
