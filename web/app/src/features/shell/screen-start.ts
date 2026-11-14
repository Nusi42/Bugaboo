import { useLayout } from "@bugaboo/ui";

import { topicGroups, type Place, type TopicGroup } from "../../shared/places.ts";
import { usePermissions } from "../../shared/use-me.ts";

/** Where a screen starts: on a computer a frame of settings opens at its first topic. */
export function startOf(
  path: Place["path"],
  groups: readonly TopicGroup[],
  phone: boolean,
): string {
  const first = groups[0]?.topics[0];
  return phone || first === undefined ? path : first.path;
}

/**
 * Where a link to a screen of the menu goes: the first topic of a frame of settings on a computer,
 * so a link to the frame from its first topic is a link to the page that is open (which takes no
 * second step in the history).
 */
export function useScreenStart(path: Place["path"]): string {
  const phone = useLayout() === "phone";
  return startOf(path, topicGroups(path, usePermissions()), phone);
}
