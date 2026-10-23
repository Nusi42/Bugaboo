import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { personName, personNameInside, type Person } from "../../shared/person.ts";
import type { ProjectLabel } from "./levels.ts";
import { projectText } from "./origin-text.ts";

type Reason = Schema<"ReasonBody">;
type Row = Schema<"ReasonRowBody">;
type Role = Schema<"NamedRef">;
type PlaceOfLevel = Schema<"LevelPlaceBody">;

/** What one role says about a field or a kind, as *Check access* answers it. */
export interface RoleSays {
  readonly role: Role;
  readonly level: string;
  readonly set: PlaceOfLevel;
}

/** The sentence about the row that gives a role: who has it, and where. */
function roleSentence(person: Person, row: Row, role: Role): string {
  const places = row.project ?? undefined;
  const who = personName(person);
  if (row.who.kind === "group") {
    const group = row.who.group.name;
    return places === undefined
      ? t("check-access.reason.role-group-organization", { person: who, group, role: role.name })
      : t("check-access.reason.role-group-project", {
          person: who,
          group,
          role: role.name,
          project: projectText(places),
        });
  }
  return places === undefined
    ? t("check-access.reason.role-person-organization", { person: who, role: role.name })
    : t("check-access.reason.role-person-project", {
        person: who,
        role: role.name,
        project: projectText(places),
      });
}

/** The sentence that says why a role given higher up counts in the place that was checked. */
function inheritedSentence(row: Row, place: ProjectLabel | null | undefined): string | undefined {
  if (!row.inherited) {
    return undefined;
  }
  if (row.project === undefined || row.project === null) {
    return t("check-access.reason.inherited-organization");
  }
  return place === undefined || place === null
    ? undefined
    : t("check-access.reason.inherited-project", {
        place: projectText(place),
        project: projectText(row.project),
      });
}

/** The sentence about the *No access* that wins over a role. */
function blockedSentence(person: Person, row: Row): string {
  const project = row.project === undefined || row.project === null ? "" : projectText(row.project);
  const who = personNameInside(person);
  return row.who.kind === "group"
    ? t("check-access.reason.blocked-group", { person: who, group: row.who.group.name, project })
    : t("check-access.reason.blocked-person", { person: who, project });
}

/** The sentences of one reason, some of which may not apply. */
function partsOf(
  person: Person,
  place: ProjectLabel | null | undefined,
  reason: Reason,
): readonly (string | undefined)[] {
  if (reason.kind === "given-by-role") {
    return [roleSentence(person, reason.row, reason.role), inheritedSentence(reason.row, place)];
  }
  if (reason.kind === "removed-by-no-access") {
    return [
      roleSentence(person, reason.row, reason.role),
      blockedSentence(person, reason.no_access),
    ];
  }
  if (reason.kind === "kept-from-removed-row") {
    return [
      roleSentence(person, reason.row, reason.role),
      blockedSentence(person, reason.no_access),
      t("check-access.reason.kept"),
    ];
  }
  if (reason.kind === "counts-only-for-the-organization") {
    return [t("check-access.reason.organization-only", { role: reason.role.name })];
  }
  if (reason.kind === "narrowed-by-key") {
    return [t("check-access.reason.narrowed-by-key")];
  }
  if (reason.kind === "outside-key-projects") {
    return [t("check-access.reason.outside-key")];
  }
  return [t("check-access.reason.none", { person: personNameInside(person) })];
}

/**
 * The reason a permission is given or not, in sentences (SEC-03): who has which role where, why
 * it counts here, and what stands in its way. A project the person who asks may not know about
 * is *a project you cannot see*, as the server says it.
 */
export function reasonSentences(
  person: Person,
  place: ProjectLabel | null | undefined,
  reason: Reason,
): readonly string[] {
  return partsOf(person, place, reason).filter((part): part is string => part !== undefined);
}

/**
 * The sentences that say why a field or a kind has its level for the person. They do not name
 * the field or kind, so the same reason reads the same for every one that has it and can be told
 * once for a group.
 */
export function levelSentences(
  person: Person,
  levelName: (level: string) => string,
  says: readonly RoleSays[],
  cappedByKey: boolean,
): readonly string[] {
  if (says.length === 0) {
    return [t("check-access.level.no-role", { person: personName(person) })];
  }
  const sentences = says.map((each) => {
    const level = levelName(each.level);
    if (each.set.kind === "project") {
      return t("check-access.level.role-project", {
        role: each.role.name,
        level,
        project: projectText(each.set.project),
      });
    }
    return each.set.kind === "role-own"
      ? t("check-access.level.role-organization", { role: each.role.name, level })
      : t("check-access.level.role-unset", { role: each.role.name });
  });
  return [
    ...sentences,
    ...(says.length > 1 ? [t("check-access.level.highest")] : []),
    ...(cappedByKey ? [t("check-access.level.capped")] : []),
  ];
}
