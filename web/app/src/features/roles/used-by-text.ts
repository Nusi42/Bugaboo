import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { listText } from "../../shared/list-text.ts";

/** Whether anybody has the role in the places the viewer sees. */
export function isUsed(used: Schema<"RoleUsedByBody">): boolean {
  return used.whole_organization || used.projects > 0 || used.people > 0 || used.groups > 0;
}

/** What a role is used for, in a sentence: *Used for the whole organization and in 4 projects.* */
export function roleUsedText(used: Schema<"RoleUsedByBody">): string {
  const parts: string[] = [];
  if (used.whole_organization) {
    parts.push(t("roles.used-whole"));
  }
  if (used.projects > 0) {
    parts.push(t("roles.used-projects", { count: used.projects }));
  }
  if (parts.length === 0) {
    return t("roles.used-nothing");
  }
  const places = listText(parts);
  if (used.people === 0 && used.groups === 0) {
    return t("roles.used-by-nobody", { places });
  }
  return t("roles.used", { places, people: used.people, groups: used.groups });
}
