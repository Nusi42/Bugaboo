import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";

/** Where a role given to somebody counts, as the API names it. */
export type WhereItCounts = Schema<"AppliesToBody">;

/** Where a row of *Who has which role* is: the whole organization, a project, or one that is not named. */
export type RowPlace = Schema<"PlaceRef">;

const appliesTexts: Readonly<Record<WhereItCounts, MessageKey>> = {
  organization: "roles.applies.organization",
  project: "roles.applies.project",
  "project-and-sub-projects": "roles.applies.project-and-sub-projects",
};

/** *Where it counts*, in the words of the glossary. */
export function appliesToText(appliesTo: WhereItCounts): string {
  return t(appliesTexts[appliesTo]);
}

/** Where a row is, in a few words: *Whole organization*, the name of the project, or the hidden one. */
export function placeText(place: RowPlace): string {
  if (place.kind === "project") {
    return place.name;
  }
  return t(place.kind === "organization" ? "roles.place.organization" : "roles.place.hidden");
}

/** The role of a row, or *No access* for a row without one. */
export function roleText(role: Schema<"RoleRef"> | null | undefined): string {
  return role === null || role === undefined ? t("roles.no-access") : role.name;
}

/** What a row says about the role and where it is: *Team member in Website*. */
export function heldText(held: Schema<"RoleAtRef">): string {
  return t("roles.held", { role: roleText(held.role), place: placeText(held.place) });
}

/** The sentence of a permission, as the catalogue says it (`permission.<key>`). */
export function permissionSentence(permission: Schema<"Permission">): MessageKey {
  return `permission.${permission}`;
}

/** The heading of a topic of the permissions (`permission-topic.<topic>`). */
export function topicHeading(topic: Schema<"Topic">): MessageKey {
  return `permission-topic.${topic}`;
}
