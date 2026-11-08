import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

import { useProjectChoices } from "./agents.ts";
import { projectChoicesOf, type ProjectKey } from "./project-choices.ts";

export interface ProjectSelectProps {
  readonly label: string;
  readonly description?: string;
  /** What a choice holds: the ID of the project, or its short name. */
  readonly valueKey: ProjectKey;
  readonly value: string;
  readonly onChange: (project: string) => void;
}

/** A choice among the projects the person can name: the one select of the windows that need a project. */
export function ProjectSelect({
  label,
  description,
  valueKey,
  value,
  onChange,
}: ProjectSelectProps) {
  const projects = projectChoicesOf(useProjectChoices().data?.items ?? [], valueKey);
  return (
    <SelectField
      label={label}
      {...(description === undefined ? {} : { description })}
      value={value}
      onChange={onChange}
      choices={[{ value: "", label: t("agents.rule-choose") }, ...projects]}
    />
  );
}
