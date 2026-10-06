import type { MessageKey } from "@bugaboo/messages";

import {
  paths,
  projectPageAddress,
  projectRolesAddress,
  projectSettingsAddress,
} from "../../shared/paths.ts";
import type { OneProject } from "./use-projects.ts";

/** The pages of the settings of a project (words of the code, not text for people). */
export const settingsTab = {
  general: "general",
  roles: "roles",
  fields: "fields",
  people: "people",
  check: "check",
} as const;

/** One of the tabs. */
export type SettingsTab = (typeof settingsTab)[keyof typeof settingsTab];

/** What each tab is called, in the order they are drawn. */
const tabNames: Record<SettingsTab, MessageKey> = {
  [settingsTab.general]: "projects.tab-general",
  [settingsTab.roles]: "roles.project-link",
  [settingsTab.fields]: "nav.field-permissions",
  [settingsTab.people]: "nav.people-visibility",
  [settingsTab.check]: "nav.check-access",
};

/** The name of a tab: the page's name in the menu and in the trail. */
export function tabName(tab: SettingsTab): MessageKey {
  return tabNames[tab];
}

/** How the address of each tab is made from the short name of the project. */
const tabAddresses: Record<SettingsTab, (shortName: string) => string> = {
  [settingsTab.general]: projectSettingsAddress,
  [settingsTab.roles]: projectRolesAddress,
  [settingsTab.fields]: (shortName) => projectPageAddress(paths.projectFields, shortName),
  [settingsTab.people]: (shortName) => projectPageAddress(paths.projectPeople, shortName),
  [settingsTab.check]: (shortName) => projectPageAddress(paths.projectCheckAccess, shortName),
};

/** The address of a tab of a project, with its short name filled in. */
export function tabAddress(tab: SettingsTab, shortName: string): string {
  return tabAddresses[tab](shortName);
}

/** Which tabs of the settings the person may open in this project, in the order they are drawn. */
export function tabsOf(project: OneProject): readonly SettingsTab[] {
  const may = (permission: OneProject["permissions"][number]) =>
    project.permissions.includes(permission);
  const tabs: SettingsTab[] = [];
  if (may("change-project-settings") || project.may_move !== "nowhere") {
    tabs.push(settingsTab.general);
  }
  if (may("decide-roles")) {
    tabs.push(settingsTab.roles);
  }
  if (may("decide-what-roles-see")) {
    tabs.push(settingsTab.fields, settingsTab.people);
  }
  if (may("check-access")) {
    tabs.push(settingsTab.check);
  }
  return tabs;
}
