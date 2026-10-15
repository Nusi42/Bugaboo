import { t } from "@bugaboo/messages";

import type { OwnField } from "./fields.ts";

/** The name of a field as its box shows it: a field that must be filled in says so (*Customer (required)*). */
export function labelOf(field: OwnField): string {
  return field.rules.required === true
    ? t("fields.label-required", { field: field.name })
    : field.name;
}

/** The words every box of a field carries: its name, its help text and what is wrong with it. */
export function boxWords(field: OwnField, error?: string) {
  return {
    label: labelOf(field),
    ...(field.help === "" ? {} : { description: field.help }),
    ...(error === undefined ? {} : { errorMessage: error }),
  };
}
