import { t } from "@bugaboo/messages";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo } from "react";

import { paths, projectSettingsAddress } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import type { TrailStep } from "../shell/index.ts";
import { useProjectTrail } from "./project-trail.ts";
import { useProject } from "./use-projects.ts";

/** The pages of a project (words of the code, not text for people). */
export const projectPage = { overview: "overview", settings: "settings" } as const;

/** Which page of a project is open. */
export type ProjectPageKind = (typeof projectPage)[keyof typeof projectPage];

/** The trail step of the settings page, after the project itself. */
function settingsStep(shortName: string): TrailStep {
  return { label: t("projects.settings-link"), href: projectSettingsAddress(shortName) };
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
  const settings = kind === projectPage.settings;
  usePageTitle(
    settings
      ? t("projects.settings-title", { name: found?.project.name ?? "" })
      : (found?.project.name ?? t("nav.projects")),
  );
  const last = useMemo(
    () => (settings ? settingsStep(current ?? address) : undefined),
    [settings, current, address],
  );
  useProjectTrail(found, last);
  useEffect(() => {
    if (current !== undefined && current !== address) {
      void navigate({
        to: settings ? paths.projectSettings : paths.project,
        params: { project: current },
        replace: true,
      });
    }
  }, [current, address, settings, navigate]);
  return project;
}
