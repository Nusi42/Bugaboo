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

/** The name in a filter `<field> is <name>`, without the quotes the language puts around some names. */
function nameIn(filter: string | null | undefined, by: string): string | undefined {
  const start = `${by} is `;
  if (filter === null || filter === undefined || !filter.startsWith(start)) {
    return undefined;
  }
  return filter.slice(start.length).replace(/^"(.*)"$/u, "$1");
}

/** The ID of a group of things the server names by an ID: the kind and the ID are its place. */
function placed(group: FoundGroup, by: string): Pick<GroupOfKey, "id" | "name"> {
  const { key } = group;
  if (key.type === "tag" || key.type === "label") {
    return { id: `${key.type}:${key.value.id}`, name: key.value.name ?? undefined };
  }
  if (key.type === "person") {
    return { id: `person:${group.filter ?? "-"}`, name: personName(key.value) };
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
  return { id: `${key.type}:${key.value}`, name: nameIn(group.filter, by) };
}

/** What the header says of a group: its words, the status group or priority it is, and its place. */
function shown(
  group: FoundGroup,
  by: string,
): Pick<GroupOfKey, "id" | "statusGroup" | "priority" | "name"> {
  const { key } = group;
  return {
    ...placed(group, by),
    statusGroup: key.type === "status-group" ? key.value : undefined,
    priority: key.type === "priority" ? key.value : undefined,
  };
}

/** The group the server sends for the field `by`: its header, and the filter that selects its issues. */
export function groupOfKey(group: FoundGroup, by: string): GroupOfKey {
  return { ...shown(group, by), filter: group.filter ?? undefined };
}
