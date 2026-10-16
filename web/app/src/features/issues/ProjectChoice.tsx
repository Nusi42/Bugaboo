import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part, PickerOption, SearchPicker } from "@bugaboo/ui";
import { useState } from "react";

import { pageSize } from "../../shared/paged.ts";
import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { linesOf, useProjectList } from "../projects/index.ts";

export interface ProjectChoiceProps {
  /** The name of the project that is chosen, if one is. */
  readonly name: string | undefined;
  /** The short name of the project that is chosen. */
  readonly shortName: string | undefined;
  /** Called with the short name of the project that is picked. */
  readonly onPick: (shortName: string) => void;
}

/** The projects the person finds by searching, one to pick. */
function Picker({ shortName, onPick }: Pick<ProjectChoiceProps, "shortName" | "onPick">) {
  const { words, group, frame } = usePickerFrame();
  const found = useProjectList({ search: words }, pageSize.short);
  const projects = linesOf(found.data?.pages);
  return (
    <SearchPicker
      legend={t("issues.project")}
      searchLabel={t("projects.picker-search")}
      listLabel={t("projects.picker-list")}
      {...frame}
      {...pickerStatus(found, projects.length, {
        nothing: t("projects.picker-nothing"),
        more: t("projects.picker-more"),
      })}
    >
      {projects.map((project) => (
        <PickerOption
          key={project.id}
          group={group}
          checked={project.shortName === shortName}
          label={project.name}
          hint={project.shortName}
          onPick={() => {
            onPick(project.shortName);
          }}
        />
      ))}
    </SearchPicker>
  );
}

/**
 * The first question of the new-issue form (UX §4.5): which project. A project that is known (the
 * one the person was looking at) is named with a button to change it; without one the picker is
 * shown.
 */
export function ProjectChoice({ name, shortName, onPick }: ProjectChoiceProps) {
  const [changing, setChanging] = useState(false);
  if (name !== undefined && !changing) {
    return (
      <div>
        <p {...partOf(part.fieldLabel)}>{t("issues.project")}</p>
        <span {...partOf(part.row)}>
          <strong {...partOf(part.strong)}>{name}</strong>
          <Button
            look={buttonLook.quiet}
            onPress={() => {
              setChanging(true);
            }}
          >
            {t("issues.change")}
          </Button>
        </span>
      </div>
    );
  }
  return (
    <Picker
      shortName={shortName}
      onPick={(picked) => {
        onPick(picked);
        setChanging(false);
      }}
    />
  );
}
