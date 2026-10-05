import { t } from "@bugaboo/messages";
import { TextField } from "@bugaboo/ui";

import { asShortName } from "./short-name.ts";

export interface ProjectFieldProps {
  readonly value: string;
  readonly onChange: (value: string) => void;
}

/** What the project is called. */
export function ProjectNameField({ value, onChange }: ProjectFieldProps) {
  return (
    <TextField
      label={t("projects.field-name")}
      description={t("projects.field-name-help")}
      requiredMessage={t("validation.required")}
      value={value}
      onChange={onChange}
    />
  );
}

export interface ShortNameFieldProps extends ProjectFieldProps {
  /** The project has a short name already, which stays with it when it is changed. */
  readonly changing?: boolean;
}

/** The short name: letters typed in are turned into capitals. */
export function ShortNameField({ value, onChange, changing = false }: ShortNameFieldProps) {
  const rule = t("projects.field-short-name-help");
  const description = changing ? `${rule} ${t("projects.short-name-change-help")}` : rule;
  return (
    <TextField
      label={t("projects.field-short-name")}
      description={description}
      requiredMessage={t("validation.required")}
      value={value}
      onChange={(typed) => {
        onChange(asShortName(typed));
      }}
    />
  );
}
