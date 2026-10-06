import { t } from "@bugaboo/messages";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo } from "react";

import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import type { TrailStep } from "../shell/index.ts";
import { useProjectTrail } from "./project-trail.ts";
import { settingsTab, tabAddress, tabName, type SettingsTab } from "./settings-tabs.ts";
import { useProject } from "./use-projects.ts";

/** The pages of a project (words of the code, not text for people). */
export const projectPage = {
  overview: "overview",
  settings: "settings",
  roles: "roles",
  fields: "fields",
  people: "people",
  check: "check",
} as const;

/** Which page of a project is open. */
export type ProjectPageKind = (typeof projectPage)[keyof typeof projectPage];

/** The tab that a page of the settings of a project is, if it is one. */
const tabOfPage: Readonly<Partial<Record<ProjectPageKind, SettingsTab>>> = {
  [projectPage.settings]: settingsTab.general,
  [projectPage.roles]: settingsTab.roles,
  [projectPage.fields]: settingsTab.fields,
  [projectPage.people]: settingsTab.people,
  [projectPage.check]: settingsTab.check,
};

/** The trail steps of a page of the settings, after the project itself: the tab, below *Project settings*. */
function stepsOf(kind: ProjectPageKind, shortName: string): readonly TrailStep[] | undefined {
  const tab = tabOfPage[kind];
  if (tab === undefined) {
    return undefined;
  }
  const settings = {
    label: t("projects.settings-link"),
    href: tabAddress(settingsTab.general, shortName),
  };
  return tab === settingsTab.general
    ? [settings]
    : [settings, { label: t(tabName(tab)), href: tabAddress(tab, shortName) }];
}

/** The page where a project that was asked for by an old short name is found again. */
function addressOf(kind: ProjectPageKind): string {
  return {
    [projectPage.overview]: paths.project,
    [projectPage.settings]: paths.projectSettings,
    [projectPage.roles]: paths.projectRoles,
    [projectPage.fields]: paths.projectFields,
    [projectPage.people]: paths.projectPeople,
    [projectPage.check]: paths.projectCheckAccess,
  }[kind];
}

/** The title of the tab of the browser for a page of a project. */
function titleOf(kind: ProjectPageKind, name: string): string {
  return {
    [projectPage.overview]: name === "" ? t("nav.projects") : name,
    [projectPage.settings]: t("projects.settings-title", { name }),
    [projectPage.roles]: t("assignments.title-project", { name }),
    [projectPage.fields]: t("field-permissions.project-title", { project: name }),
    [projectPage.people]: t("people-visibility.project-title", { project: name }),
    [projectPage.check]: t("check-access.project-title", { project: name }),
  }[kind];
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
  usePageTitle(titleOf(kind, name));
  const last = useMemo(() => stepsOf(kind, current ?? address), [kind, current, address]);
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
