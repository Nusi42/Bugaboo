import { decimalParts, type Schema } from "@bugaboo/api-client";

import { addDays } from "../../shared/format.ts";
import { ownValueText } from "../../shared/own-value-text.ts";
import { personName } from "../../shared/person.ts";
import type { OwnColumn } from "./own-columns.ts";

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
  /** How many issues the search finds in it. */
  readonly count: number;
  /** The name of the group where the key holds one; none where the screen words it itself. */
  readonly name: string | undefined;
}

/** The day the store counts days from. */
const epoch = "1970-01-01";

/**
 * What a group of a field of whole numbers holds, as the value the field has on the issue page: a
 * number, seconds, a flag, a day or a moment (the store counts a day as days and a moment as
 * microseconds).
 */
function valueOfWhole(kind: Schema<"FieldType">, whole: number): unknown {
  if (kind === "decimal") {
    return whole / decimalParts;
  }
  if (kind === "yes-no") {
    return whole !== 0;
  }
  if (kind === "date") {
    return addDays(epoch, whole);
  }
  return kind === "date-time" ? new Date(whole / 1000).toISOString() : whole;
}

/** What tells a person's group from the others: the ID of a person shown by name, else the words that show them. */
function personPlace(person: Schema<"PersonRef">): string {
  if (person.shown === "name") {
    return person.id;
  }
  if (person.shown === "role") {
    return `role:${person.role}`;
  }
  return person.shown === "former" ? `former:${person.code}` : "someone";
}

/**
 * The place and the name of a group: what tells it from the others, and what its header says where
 * the server gave it a name (`name`, found by the one resolution of names), a person as the viewer
 * sees them, or a tag or a label by its name.
 */
function placed(group: FoundGroup, column: OwnColumn | undefined): Pick<GroupOfKey, "id" | "name"> {
  const { key } = group;
  if (key.type === "tag" || key.type === "label") {
    return { id: `${key.type}:${key.value.id}`, name: key.value.name ?? undefined };
  }
  if (key.type === "person") {
    return {
      id: `person:${personPlace(key.value)}`,
      name: personName(key.value),
    };
  }
  if (key.type === "empty") {
    return { id: "empty", name: undefined };
  }
  if (key.type === "whole") {
    const name =
      column === undefined
        ? undefined
        : ownValueText(column.kind, valueOfWhole(column.kind, key.value));
    return { id: `whole:${String(key.value)}`, name: name ?? String(key.value) };
  }
  if (key.type === "text") {
    return { id: `text:${key.value}`, name: key.value };
  }
  if (key.type === "priority" || key.type === "status-group") {
    return { id: `${key.type}:${key.value}`, name: undefined };
  }
  return { id: `${key.type}:${key.value}`, name: group.name ?? undefined };
}

/** The group the server sends: its header, and the filter that selects its issues. */
export function groupOfKey(group: FoundGroup, column?: OwnColumn): GroupOfKey {
  const { key } = group;
  return {
    ...placed(group, column),
    statusGroup: key.type === "status-group" ? key.value : undefined,
    priority: key.type === "priority" ? key.value : undefined,
    filter: group.filter ?? undefined,
    count: group.issues,
  };
}

/**
 * The groups the server sends for a field, each with a place of its own: two groups that show the
 * same words (two people of one role) get the number of their turn after it.
 */
export function groupsOfKeys(
  groups: readonly FoundGroup[],
  column?: OwnColumn,
): readonly GroupOfKey[] {
  const seen = new Map<string, number>();
  return groups.map((each) => {
    const group = groupOfKey(each, column);
    const turn = seen.get(group.id) ?? 0;
    seen.set(group.id, turn + 1);
    return turn === 0 ? group : { ...group, id: `${group.id}#${String(turn)}` };
  });
}
