import { t } from "@bugaboo/messages";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo } from "react";

import { paths, projectRolesAddress, projectSettingsAddress } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import type { TrailStep } from "../shell/index.ts";
import { useProjectTrail } from "./project-trail.ts";
import { useProject } from "./use-projects.ts";

/** The pages of a project (words of the code, not text for people). */
export const projectPage = {
  overview: "overview",
  settings: "settings",
  roles: "roles",
} as const;

/** Which page of a project is open. */
export type ProjectPageKind = (typeof projectPage)[keyof typeof projectPage];

/** The trail step of the settings page, after the project itself. */
function settingsStep(shortName: string): TrailStep {
  return { label: t("projects.settings-link"), href: projectSettingsAddress(shortName) };
}

/** The trail step of *Who has which role*, after the project itself. */
function rolesStep(shortName: string): TrailStep {
  return { label: t("roles.project-link"), href: projectRolesAddress(shortName) };
}

/** The address of a kind of page of a project, for the one that names a short name. */
function addressOf(
  kind: ProjectPageKind,
): typeof paths.project | typeof paths.projectSettings | typeof paths.projectRoles {
  if (kind === projectPage.settings) {
    return paths.projectSettings;
  }
  return kind === projectPage.roles ? paths.projectRoles : paths.project;
}

/**
 * What every page of a project does: asks for the project by the address, names the tab and the
 * trail of *where you are*, and replaces an address that names an old short name by the current
 * one (the old address keeps working, M2-Q4).
 */
export function useProjectPage(address: string, kind: ProjectPageKind) {
  const project = useProject(address);
  const navigate = useNavigate();
  const found = project.data;
  // What stands in for a project that is still loading names no address to go to.
  const current = project.isPlaceholderData ? undefined : found?.project.short_name;
  const name = found?.project.name ?? "";
  usePageTitle(
    {
      [projectPage.settings]: t("projects.settings-title", { name }),
      [projectPage.roles]: t("assignments.title-project", { name }),
      [projectPage.overview]: found?.project.name ?? t("nav.projects"),
    }[kind],
  );
  const last = useMemo(() => {
    if (kind === projectPage.settings) {
      return settingsStep(current ?? address);
    }
    return kind === projectPage.roles ? rolesStep(current ?? address) : undefined;
  }, [kind, current, address]);
  useProjectTrail(found, last);
  useEffect(() => {
    if (current !== undefined && current !== address) {
      void navigate({
        to: addressOf(kind),
        params: { project: current },
        replace: true,
      });
    }
  }, [current, address, kind, navigate]);
  return project;
}
