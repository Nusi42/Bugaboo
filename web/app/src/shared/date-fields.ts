import type { Schema } from "@bugaboo/api-client";

/** The words of the two dates of an issue (PLAN-01), as the layouts, levels and history name them: written once. */
export const dateField = {
  start: "start-date",
  due: "due-date",
} as const satisfies Record<string, Schema<"Field">>;

/** One of the two dates of an issue, by its word. */
export type DateField = (typeof dateField)[keyof typeof dateField];
