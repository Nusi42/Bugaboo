import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** A status change of some workflow of a project, as the server lists them: where it leads and its words. */
export type ProjectChange = Schema<"ProjectStatusChangeBody">;

/** What tells one status change from another: the status it leads to and the words of its button. */
interface Choosable {
  readonly to: { readonly key: string; readonly name: string };
  readonly button: string;
}

/**
 * The ID of a status change among those offered. Two can lead to one status with other words
 * (*Stop working*, *Reopen*), so the status alone does not name one. It is only compared, never
 * taken apart: the change is found again in the list it came from.
 */
export function choiceIdOf(change: Choosable): string {
  return JSON.stringify([change.to.key, change.button]);
}

/**
 * What a choice says: the words of the button, and the status it leads to when another choice has
 * the same words for another status, so the two can be told apart.
 */
export function choiceLabelOf(change: Choosable, offered: readonly Choosable[]): string {
  const alike = offered.some(
    (each) => each.button === change.button && each.to.key !== change.to.key,
  );
  return alike
    ? t("many-changes.change-and-status", { button: change.button, status: change.to.name })
    : change.button;
}
