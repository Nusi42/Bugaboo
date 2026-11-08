import type { Schema } from "@bugaboo/api-client";

/** A project the person can name, as the list of project choices has it. */
type ProjectChoice = Schema<"ProjectChoiceItem">;

/** What is chosen when a project is: its ID, or its short name (what the people of a project are asked by). */
export const projectKey = { id: "id", shortName: "short_name" } as const;
export type ProjectKey = (typeof projectKey)[keyof typeof projectKey];

/** The name of a project in a choice: its name, else its short name, else its ID. The one mapping. */
function labelOf(project: ProjectChoice): string {
  return project.name ?? project.short_name ?? project.id ?? "";
}

/** The projects as choices, each by `key`, with the one name; a project that has no such key is left out. */
export function projectChoicesOf(
  projects: readonly ProjectChoice[],
  key: ProjectKey,
): { readonly value: string; readonly label: string }[] {
  const choices: { readonly value: string; readonly label: string }[] = [];
  for (const project of projects) {
    const value = project[key];
    if (value !== undefined && value !== null) {
      choices.push({ value, label: labelOf(project) });
    }
  }
  return choices;
}
