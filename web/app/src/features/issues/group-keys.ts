import type { Schema } from "@bugaboo/api-client";

import { personName } from "../../shared/person.ts";

/** A group of a search as the server sends it. */
type FoundGroup = Schema<"SearchGroupBody">;

/** A group of a list: what it is called, and the filter that gives its rows. */
export interface GroupOfKey {
  /** A name that tells it from every other group of the list. */
  readonly id: string;
  /** The filter the server wrote for the group, as search text; none where the group has no name the person may use. */
  readonly filter: string | undefined;
  /** The status group it is, for the icon and the color of its header. */
  readonly statusGroup: Schema<"StatusGroup"> | undefined;
  /** The priority it is. */
  readonly priority: Schema<"Priority"> | undefined;
  /** The name of the group where the key holds one; none where the screen words it itself. */
  readonly name: string | undefined;
}

/**
 * The place and the name of a group: what tells it from the others, and what its header says where
 * the server gave it a name (`name`, found by the one resolution of names), a person as the viewer
 * sees them, or a tag or a label by its name.
 */
function placed(group: FoundGroup, at: number): Pick<GroupOfKey, "id" | "name"> {
  const { key } = group;
  if (key.type === "tag" || key.type === "label") {
    return { id: `${key.type}:${key.value.id}`, name: key.value.name ?? undefined };
  }
  if (key.type === "person") {
    return {
      id: key.value.shown === "name" ? `person:${key.value.id}` : `person:${String(at)}`,
      name: personName(key.value),
    };
  }
  if (key.type === "empty") {
    return { id: "empty", name: undefined };
  }
  if (key.type === "whole") {
    return { id: `whole:${String(key.value)}`, name: String(key.value) };
  }
  if (key.type === "text") {
    return { id: `text:${key.value}`, name: key.value };
  }
  if (key.type === "priority" || key.type === "status-group") {
    return { id: `${key.type}:${key.value}`, name: undefined };
  }
  return { id: `${key.type}:${key.value}`, name: group.name ?? undefined };
}

/** The group the server sends, the `at`-th of the field: its header, and the filter that selects its issues. */
export function groupOfKey(group: FoundGroup, at: number): GroupOfKey {
  const { key } = group;
  return {
    ...placed(group, at),
    statusGroup: key.type === "status-group" ? key.value : undefined,
    priority: key.type === "priority" ? key.value : undefined,
    filter: group.filter ?? undefined,
  };
}
