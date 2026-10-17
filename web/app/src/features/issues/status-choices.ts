import type { Schema } from "@bugaboo/api-client";

/** A status change of some workflow of a project, as the server lists them: where it leads and its words. */
export type ProjectChange = Schema<"ProjectStatusChangeBody">;

/** Between the key of the status and the words of the button in the ID of a choice. */
const SEPARATOR = "|";

/**
 * The ID of a choice of a status change for many issues. Two status changes can lead to the same
 * status with other words (*Stop working*, *Reopen*), so the status key alone does not name one.
 */
export function choiceIdOf(change: ProjectChange): string {
  return `${change.to.key}${SEPARATOR}${change.button}`;
}

/** The key of the status that a choice leads to. */
export function statusKeyOfChoice(id: string): string {
  return id.split(SEPARATOR, 1)[0] ?? id;
}
