import { t } from "@bugaboo/messages";

import type { Origin, ProjectLabel } from "./levels.ts";

/** A project as a name in a sentence: its name, or *a project you can't see*. */
export function projectText(project: ProjectLabel): string {
  return project.shown === "visible"
    ? (project.name ?? project.short_name ?? "")
    : t("access.hidden-project");
}

/** Where the level of a cell was set, in words; nothing for a role's own level on the organization's table. */
export function originText(origin: Origin, inProject: boolean): string | undefined {
  if (origin.kind === "set_here") {
    return t("access.origin-here");
  }
  if (origin.kind === "inherited") {
    return t("access.origin-from", { project: projectText(origin.from) });
  }
  if (origin.kind === "unset") {
    return t("access.origin-unset");
  }
  return inProject ? t("access.origin-role-own") : undefined;
}
