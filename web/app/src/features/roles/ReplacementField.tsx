import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

import { roleLinesOf, useRoles } from "./roles.ts";

export interface ReplacementFieldProps {
  /** The role that is deleted: it cannot take its own place. */
  readonly id: string;
  /** The ID of the role chosen, or an empty text when none is chosen yet. */
  readonly value: string;
  readonly onChange: (value: string) => void;
}

/** The roles that can take the place of a deleted one, to choose from. */
export function ReplacementField({ id, value, onChange }: ReplacementFieldProps) {
  const roles = useRoles();
  const choices = [
    { value: "", label: t("roles.replace-choose") },
    ...roleLinesOf(roles.data?.pages)
      .filter((role) => role.id !== id)
      .map((role) => ({ value: role.id, label: role.name })),
  ];
  return (
    <SelectField
      label={t("roles.replace-field")}
      description={t("roles.replace-help")}
      choices={choices}
      value={value}
      onChange={onChange}
    />
  );
}
