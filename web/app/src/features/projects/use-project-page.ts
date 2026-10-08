import { t } from "@bugaboo/messages";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo } from "react";

import { routeParam } from "../../shared/paths.ts";
import { projectPageAddress, projectPages, type ProjectPage } from "../../shared/project-pages.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import type { TrailStep } from "../shell/index.ts";
import { useProjectTrail } from "./project-trail.ts";
import { useProject } from "./use-projects.ts";

/** The trail steps of a page of the settings, after the project itself: the tab, below *Project settings*. */
function stepsOf(page: ProjectPage, shortName: string): readonly TrailStep[] | undefined {
  if (page.tab === null) {
    return undefined;
  }
  const settings = {
    label: t(projectPages.settings.name),
    href: projectPageAddress(projectPages.settings, shortName),
  };
  return page === projectPages.settings
    ? [settings]
    : [settings, { label: t(page.tab), href: projectPageAddress(page, shortName) }];
}

/**
 * What every page of a project does: asks for the project by the address, names the tab and the
 * trail of *where you are*, and replaces an address that names an old short name by the current
 * one (the old address keeps working, M2-Q4).
 */
export function useProjectPage(address: string, page: ProjectPage) {
  const project = useProject(address);
  const navigate = useNavigate();
  const found = project.data;
  // What stands in for a project that is still loading names no address to go to.
  const current = project.isPlaceholderData ? undefined : found?.project.short_name;
  const name = found?.project.name ?? "";
  usePageTitle(page.title(name));
  const last = useMemo(() => stepsOf(page, current ?? address), [page, current, address]);
  useProjectTrail(found, last);
  useEffect(() => {
    if (current !== undefined && current !== address) {
      void navigate({
        to: page.path,
        params: { [routeParam.project]: current },
        replace: true,
      });
    }
  }, [current, address, page, navigate]);
  return project;
}
