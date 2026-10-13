import type { OwnField } from "./fields.ts";

/** The words every box of a field carries: its name, its help text and what is wrong with it. */
export function boxWords(field: OwnField, error?: string) {
  return {
    label: field.name,
    ...(field.help === "" ? {} : { description: field.help }),
    ...(error === undefined ? {} : { errorMessage: error }),
  };
}
