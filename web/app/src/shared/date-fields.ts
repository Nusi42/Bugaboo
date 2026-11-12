import type { Schema } from "@bugaboo/api-client";
import type { MessageKey } from "@bugaboo/messages";

/** The words of the two dates of an issue (PLAN-01), as the layouts, levels and history name them: written once. */
export const dateField = {
  start: "start-date",
  due: "due-date",
} as const satisfies Record<string, Schema<"Field">>;

/** One of the two dates of an issue, by its word. */
export type DateField = (typeof dateField)[keyof typeof dateField];

/** The name of each of the two dates, which the rows, the form and the sentences write. */
export const dateFieldName: Readonly<Record<DateField, MessageKey>> = {
  [dateField.start]: "issue-field.start-date",
  [dateField.due]: "issue-field.due-date",
};

/** Whether a field of a form or a layout is one of the two dates. */
export function isDateField(field: string): field is DateField {
  return field === dateField.start || field === dateField.due;
}
