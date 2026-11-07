import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { listText } from "../../../shared/list-text.ts";
import { choiceName, type Choice } from "./choice.ts";

/**
 * What an access key or an app may do, in words: the choice, or how many single permissions. The one
 * text of the list of keys and the list of connected apps.
 */
export function mayText(choice: Choice | null | undefined, permissionCount: number): string {
  if (choice === undefined || choice === null) {
    return "";
  }
  return choice === "permissions"
    ? t("access-keys.row.permissions", { count: permissionCount })
    : t(choiceName(choice));
}

/** Where a key or an app works, in words: the projects it is limited to, or every project. */
export function whereText(
  projects: readonly Schema<"ProjectLabelBody">[] | null | undefined,
): string {
  if (projects === undefined || projects === null || projects.length === 0) {
    return t("access-keys.row.projects-all");
  }
  const names = projects.map((project) =>
    project.shown === "visible"
      ? t("access-keys.project-label", {
          short_name: project.short_name ?? "",
          name: project.name ?? "",
        })
      : t("access-keys.row.hidden-project"),
  );
  return t("access-keys.row.projects-some", { names: listText(names) });
}
