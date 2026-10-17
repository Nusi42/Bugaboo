import { iconName } from "@bugaboo/ui/icons";

import { fieldPlaces } from "./field-places.ts";
import { labelPlaces } from "./label-places.ts";
import { layoutPlaces } from "./layout-places.ts";
import { paths } from "./paths.ts";
import type { Place } from "./place.ts";
import { relationPlaces } from "./relation-places.ts";

/** The topics of *Organization settings*, and the screens reached from them. */
const organizationTopics: readonly Place[] = [
  {
    path: paths.people,
    name: "nav.people",
    icon: iconName.people,
    parent: paths.organizationSettings,
    anyOf: ["manage-people", "manage-invitations", "change-organization-settings"],
    group: "nav.group.people",
    listed: true,
  },
  {
    // A person's page is reached from the list of people.
    path: paths.person,
    name: "nav.people",
    icon: iconName.people,
    parent: paths.people,
    anyOf: ["manage-people"],
    listed: false,
  },
  {
    path: paths.groups,
    name: "nav.groups",
    icon: iconName.groups,
    parent: paths.organizationSettings,
    anyOf: ["manage-groups"],
    group: "nav.group.people",
    listed: true,
  },
  {
    path: paths.roles,
    name: "nav.roles",
    icon: iconName.roles,
    parent: paths.organizationSettings,
    anyOf: ["manage-roles", "decide-roles"],
    group: "nav.group.people",
    listed: true,
  },
  {
    // A role's page is reached from the list of roles.
    path: paths.role,
    name: "nav.roles",
    icon: iconName.roles,
    parent: paths.roles,
    anyOf: ["manage-roles"],
    listed: false,
  },
  {
    path: paths.signInRules,
    name: "nav.sign-in-rules",
    icon: iconName.security,
    parent: paths.organizationSettings,
    anyOf: ["change-organization-settings"],
    group: "nav.group.people",
    listed: true,
  },
  {
    path: paths.fieldPermissions,
    name: "nav.field-permissions",
    icon: iconName["field-permissions"],
    parent: paths.organizationSettings,
    anyOf: ["decide-what-roles-see"],
    group: "nav.group.people",
    listed: true,
  },
  {
    path: paths.peopleVisibility,
    name: "nav.people-visibility",
    icon: iconName["people-visibility"],
    parent: paths.organizationSettings,
    anyOf: ["decide-what-roles-see"],
    group: "nav.group.people",
    listed: true,
  },
  {
    path: paths.checkAccess,
    name: "nav.check-access",
    icon: iconName["check-access"],
    parent: paths.organizationSettings,
    anyOf: ["check-access"],
    group: "nav.group.people",
    listed: true,
  },
  {
    path: paths.themes,
    name: "nav.themes",
    icon: iconName.palette,
    parent: paths.organizationSettings,
    anyOf: ["change-organization-settings"],
    group: "nav.group.look-org",
    listed: true,
  },
  {
    path: paths.projects,
    name: "nav.projects",
    icon: iconName.projects,
    parent: paths.organizationSettings,
    anyOf: ["create-projects"],
    group: "nav.group.work",
    listed: true,
  },
  {
    path: paths.issueTypes,
    name: "nav.issue-types",
    icon: iconName["issue-types"],
    parent: paths.organizationSettings,
    anyOf: ["manage-issue-types"],
    group: "nav.group.work",
    listed: true,
  },
  ...fieldPlaces,
  ...relationPlaces,
  ...labelPlaces,
  ...layoutPlaces,
  {
    path: paths.mailServer,
    name: "nav.mail-server",
    icon: iconName.mail,
    parent: paths.organizationSettings,
    anyOf: ["change-organization-settings"],
    group: "nav.group.connections",
    listed: true,
  },
  {
    path: paths.organizationSecurity,
    name: "nav.organization-security",
    icon: iconName.security,
    parent: paths.organizationSettings,
    anyOf: ["change-organization-settings"],
    group: "nav.group.system",
    listed: true,
  },
  {
    path: paths.status,
    name: "nav.status",
    icon: iconName.status,
    parent: paths.organizationSettings,
    anyOf: ["see-status"],
    group: "nav.group.system",
    listed: true,
  },
];

/**
 * *Organization settings* opens for a person who may open at least one of its topics, so it asks
 * for the permissions of its topics and no others (one rule, not a second list to keep in step).
 */
const organizationSettings: Place = {
  path: paths.organizationSettings,
  name: "nav.organization-settings",
  icon: iconName.organization,
  parent: paths.home,
  anyOf: [
    ...new Set(
      organizationTopics
        .filter((topic) => topic.parent === paths.organizationSettings)
        .flatMap((topic) => topic.anyOf ?? []),
    ),
  ],
  listed: true,
};

/** *Organization settings* and its topics. */
export const organizationPlaces: readonly Place[] = [organizationSettings, ...organizationTopics];
