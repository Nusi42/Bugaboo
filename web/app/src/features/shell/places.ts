import type { MessageKey } from "@bugaboo/messages";
import { iconName, type IconName } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import type { PermissionKey, Permissions } from "../../shared/permissions.ts";

type Path = (typeof paths)[keyof typeof paths];

/**
 * One screen of the signed-in app, as the menus, the top bar and quick search know it. The list
 * below is the one place that says which screens exist, where they sit and what they are called
 * (SSOT): a screen that is not in it has no menu entry, and one that is has it everywhere.
 */
export interface Place {
  readonly path: Path;
  readonly name: MessageKey;
  readonly icon: IconName;
  /** The screen this one belongs under: *Back* on a phone goes there. `null` for the start page. */
  readonly parent: Path | null;
  /** Only people who hold one of these permissions for the whole organization see it. */
  readonly anyOf?: readonly PermissionKey[];
  /** For a topic of a settings frame: the heading it is listed under in the left menu. */
  readonly group?: MessageKey;
  /** Whether menus and quick search list it. A screen reached only from another one is not. */
  readonly listed: boolean;
}

export const places: readonly Place[] = [
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
    path: paths.organizationSettings,
    name: "nav.organization-settings",
    icon: iconName.organization,
    parent: paths.home,
    anyOf: ["manage-invitations", "see-status", "change-organization-settings", "make-projects"],
    listed: true,
  },
  {
    path: paths.invitations,
    name: "nav.invitations",
    icon: iconName.invitations,
    parent: paths.organizationSettings,
    anyOf: ["manage-invitations", "change-organization-settings"],
    group: "nav.group.people",
    listed: true,
  },
  {
    path: paths.projects,
    name: "nav.projects",
    icon: iconName.projects,
    parent: paths.organizationSettings,
    anyOf: ["make-projects"],
    group: "nav.group.work",
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
  {
    // The address of a project has a part to fill in: the screen is here so that it is known,
    // the pages of the projects name themselves (`usePageTrail`).
    path: paths.project,
    name: "nav.projects",
    icon: iconName.projects,
    parent: paths.home,
    listed: false,
  },
  {
    path: paths.projectSettings,
    name: "projects.settings-link",
    icon: iconName.settings,
    parent: paths.project,
    listed: false,
  },
  {
    path: paths.frames,
    name: "nav.frames",
    icon: iconName.frames,
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
