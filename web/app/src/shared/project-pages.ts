import type { MessageKey } from "@bugaboo/messages";
import { t } from "@bugaboo/messages";
import { iconName, type IconName } from "@bugaboo/ui";

import { addressInProject, paths, type Path } from "./paths.ts";
import type { PermissionKey } from "./permissions.ts";

/** One page of a project: where it is, what it is called, who may open it and how it is titled. */
export interface ProjectPage {
  readonly path: Extract<Path, `/p/${string}`>;
  /** The screen's name in menus and in the trail of *where you are*. */
  readonly name: MessageKey;
  readonly icon: IconName;
  /** The screen this one belongs under. */
  readonly parent: Path;
  /** The name on the tab when the page is a tab of the project settings; none for the overview. */
  readonly tab: MessageKey | null;
  /** The permission in the project that opens the page, if one decides it alone. */
  readonly needs: PermissionKey | null;
  /** The title of the browser tab; the name of the project is empty while it loads. */
  readonly title: (project: string) => string;
}

/**
 * The pages of a project, written once: the overview and the tabs of its settings. The routes, the
 * menus, the tabs, the trail, the titles and the rule who may open a page all read this table
 *, also for the order of the tabs (SSOT).
 */
export const projectPages = {
  overview: {
    path: paths.project,
    name: "nav.projects",
    icon: iconName.projects,
    parent: paths.home,
    tab: null,
    needs: null,
    title: (project) => (project === "" ? t("nav.projects") : project),
  },
  settings: {
    path: paths.projectSettings,
    name: "projects.settings-link",
    icon: iconName.settings,
    parent: paths.project,
    tab: "projects.tab-general",
    // Opens for a person who may change the project or move it: not one permission alone.
    needs: null,
    title: (name) => t("projects.settings-title", { name }),
  },
  roles: {
    path: paths.projectRoles,
    name: "roles.project-link",
    icon: iconName.roles,
    parent: paths.projectSettings,
    tab: "roles.project-link",
    needs: "decide-roles",
    title: (name) => t("assignments.title-project", { name }),
  },
  fields: {
    path: paths.projectFields,
    name: "nav.field-permissions",
    icon: iconName["field-permissions"],
    parent: paths.projectSettings,
    tab: "nav.field-permissions",
    needs: "decide-what-roles-see",
    title: (project) => t("field-permissions.project-title", { project }),
  },
  people: {
    path: paths.projectPeople,
    name: "nav.people-visibility",
    icon: iconName["people-visibility"],
    parent: paths.projectSettings,
    tab: "nav.people-visibility",
    needs: "decide-what-roles-see",
    title: (project) => t("people-visibility.project-title", { project }),
  },
  check: {
    path: paths.projectCheckAccess,
    name: "nav.check-access",
    icon: iconName["check-access"],
    parent: paths.projectSettings,
    tab: "nav.check-access",
    needs: "check-access",
    title: (project) => t("check-access.project-title", { project }),
  },
} as const satisfies Readonly<Record<string, ProjectPage>>;

/** The tabs of the settings of a project, in the order they are drawn. */
export const settingsTabs: readonly ProjectPage[] = [
  projectPages.settings,
  projectPages.roles,
  projectPages.fields,
  projectPages.people,
  projectPages.check,
];

/** The address of a page of a project, with the short name filled in. */
export function projectPageAddress(page: ProjectPage, shortName: string): string {
  return addressInProject(page.path, shortName);
}

/** Whether the permissions a person holds in a project open a page that one permission decides. */
export function mayOpenProjectPage(
  page: ProjectPage,
  permissions: readonly PermissionKey[],
): boolean {
  return page.needs === null || permissions.includes(page.needs);
}
