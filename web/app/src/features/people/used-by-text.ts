import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { listText } from "../../shared/list-text.ts";

/** What a group is used for, in a sentence: *Used for the whole organization and in 4 projects.* */
export function groupUsedText(used: Schema<"GroupUsedByBody">): string {
  const parts: string[] = [];
  if (used.whole_organization) {
    parts.push(t("groups.used-whole"));
  }
  if (used.projects > 0) {
    parts.push(t("groups.used-projects", { count: used.projects }));
  }
  return parts.length === 0
    ? t("groups.used-nothing")
    : t("groups.used", { places: listText(parts) });
}
