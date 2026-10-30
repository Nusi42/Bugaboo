import type { MessageKey } from "@bugaboo/messages";
import { iconName } from "@bugaboo/ui/icons";

import { organizationPlaces } from "./organization-places.ts";
import { ownPlaces } from "./own-places.ts";
import { isAtOrUnder, partsOfAddress, paths, type Path } from "./paths.ts";
import type { Permissions } from "./permissions.ts";
import type { Place } from "./place.ts";
import { projectPlaces } from "./project-places.ts";

export type { Place } from "./place.ts";

/** Every screen of the signed-in app, in the order the menus list them. */
export const places: readonly Place[] = [
  ...ownPlaces,
  ...organizationPlaces,
  ...projectPlaces,
  {
    // An issue is reached from a list, a link or quick search; its address is `/issues/WEB-142`.
    path: paths.issue,
    name: "issues.title",
    icon: iconName.issues,
    parent: paths.home,
    listed: false,
  },
  {
    // The search over every project, and a saved search: reached from the menu's *Saved searches*.
    path: paths.search,
    name: "search.page.title",
    icon: iconName.search,
    parent: paths.home,
    listed: false,
  },
  {
    path: paths.savedSearch,
    name: "search.saved-heading",
    icon: iconName.search,
    parent: paths.search,
    listed: false,
  },
];

/** The screen at an address, if there is one. */
export function placeAt(path: string): Place | undefined {
  return places.find((place) => place.path === path);
}

/**
 * Whether the screen at `path` is the one that is open: the screen itself, a page under its address (*My settings*
 * stays marked on its topics) or a screen that belongs under it (a saved search under *Search*, UX §3.3), but Home
 * only at its own address, since every address is under it.
 */
export function isCurrentPlace(path: string, pathname: string): boolean {
  if (path === paths.home) {
    return pathname === path;
  }
  const open = places.find((place) => partsOfAddress(place.path, pathname) !== undefined);
  return (
    isAtOrUnder(path, pathname) ||
    (open !== undefined && trailTo(open.path).some((place) => place.path === path))
  );
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
