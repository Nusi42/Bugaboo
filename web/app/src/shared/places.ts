import type { MessageKey } from "@bugaboo/messages";
import { iconName } from "@bugaboo/ui/icons";

import { paths, type Path } from "./paths.ts";
import type { Permissions } from "./permissions.ts";
import type { Place } from "./place.ts";
import { fieldPlaces } from "./field-places.ts";
import { labelPlaces } from "./label-places.ts";
import { projectPlaces } from "./project-places.ts";
import { workPlaces } from "./work-places.ts";
import { relationPlaces } from "./relation-places.ts";

export type { Place } from "./place.ts";

/** The screens of the person themselves: the start page and *My settings*. */
const ownPlaces: readonly Place[] = [
  { path: paths.home, name: "nav.home", icon: iconName.home, parent: null, listed: true },
  {
    path: paths.mySettings,
    name: "nav.my-settings",
    icon: iconName.settings,
    parent: paths.home,
    listed: true,
  },
  {
    path: paths.appearance,
    name: "nav.appearance",
    icon: iconName.palette,
    parent: paths.mySettings,
    group: "nav.group.look",
    listed: true,
  },
  {
    path: paths.region,
    name: "nav.region",
    icon: iconName.region,
    parent: paths.mySettings,
    group: "nav.group.look",
    listed: true,
  },
  {
    path: paths.startPage,
    name: "nav.start-page",
    icon: iconName.home,
    parent: paths.mySettings,
    group: "nav.group.look",
    listed: true,
  },
  {
    path: paths.shortcuts,
    name: "shortcut.title",
    icon: iconName.keyboard,
    parent: paths.mySettings,
    group: "nav.group.look",
    listed: true,
  },
  {
    path: paths.security,
    name: "nav.security",
    icon: iconName.security,
    parent: paths.mySettings,
    group: "nav.group.sign-in",
    listed: true,
  },
  {
    path: paths.sessions,
    name: "nav.sessions",
    icon: iconName.sessions,
    parent: paths.mySettings,
    group: "nav.group.sign-in",
    listed: true,
  },
  {
    path: paths.accessKeys,
    name: "nav.access-keys",
    icon: iconName["access-keys"],
    parent: paths.mySettings,
    group: "nav.group.sign-in",
    listed: true,
  },
];

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
  ...workPlaces,
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

export const places: readonly Place[] = [
  ...ownPlaces,
  organizationSettings,
  ...organizationTopics,
  ...projectPlaces,
  {
    // An issue is reached from a list, a link or quick search; its address is `/issues/WEB-142`.
    path: paths.issue,
    name: "issues.title",
    icon: iconName.issues,
    parent: paths.home,
    listed: false,
  },
];

/** The screen at an address, if there is one. */
export function placeAt(path: string): Place | undefined {
  return places.find((place) => place.path === path);
}

/** The screens a person may open: those that ask for a permission only for people who hold one. */
export function placesFor(held: Permissions): readonly Place[] {
  return places.filter(
    (place) => place.anyOf === undefined || place.anyOf.some((wanted) => held.includes(wanted)),
  );
}

/** Whether a person may open the screen at an address. */
export function mayOpen(path: string, held: Permissions): boolean {
  return placesFor(held).some((place) => place.path === path);
}

/** Where a person is, from the start page down to this screen: `My settings › Appearance`. */
export function trailTo(path: string): readonly Place[] {
  const trail: Place[] = [];
  for (
    let place = placeAt(path);
    place !== undefined;
    place = place.parent === null ? undefined : placeAt(place.parent)
  ) {
    trail.unshift(place);
  }
  return trail;
}

/** One heading of a settings menu with the topics under it. */
export interface TopicGroup {
  readonly heading: MessageKey;
  readonly topics: readonly Place[];
}

/** The topics under a settings screen, grouped by heading in the order they are listed. */
export function topicGroups(parent: Path, held: Permissions): readonly TopicGroup[] {
  const topics = placesFor(held).filter(
    (place) => place.parent === parent && place.group !== undefined,
  );
  const groups: TopicGroup[] = [];
  for (const [heading, group] of Map.groupBy(topics, (place) => place.group)) {
    if (heading !== undefined) {
      groups.push({ heading, topics: group });
    }
  }
  return groups;
}
