import type { Schema } from "@bugaboo/api-client";

import type { FilterValue } from "../../shared/with-filter.ts";

/** The key of a group as the server sends it. */
type GroupKey = Schema<"SearchGroupKeyBody">;

/** A group of a list: what it is called, and the filter that gives its rows. */
export interface GroupOfKey {
  /** A name that tells it from every other group of the list. */
  readonly id: string;
  /** What the filter asks of the field; none when the key cannot be written as a filter. */
  readonly filter: FilterValue | undefined;
  /** The status group it is, for the icon and the color of its header. */
  readonly statusGroup: Schema<"StatusGroup"> | undefined;
  /** The priority it is. */
  readonly priority: Schema<"Priority"> | undefined;
  /** The name of the group where the key holds one; none where the screen words it itself. */
  readonly name: string | undefined;
}

const letters = (word: string): string => word.replaceAll("-", "").toLowerCase();

/**
 * The word of the language for a status group or a priority: the choice of the field that is the
 * same word without its dashes and capitals. The server wrote the words once, next to the choice
 * (ADR-0005 item 9); the screen finds them in the choices the field lists.
 */
function choiceFor(code: string, choices: readonly string[]): string | undefined {
  return choices.find((choice) => letters(choice) === letters(code));
}

function personName(person: Extract<GroupKey, { type: "person" }>["value"]): string | undefined {
  return person.shown === "name" ? person.name : undefined;
}

const none: Omit<GroupOfKey, "id" | "filter"> = {
  statusGroup: undefined,
  priority: undefined,
  name: undefined,
};

/** A group the server names by a bare ID: the screen does not know its name. */
function unnamed(type: string, value: string): GroupOfKey {
  return { ...none, id: `${type}:${value}`, filter: undefined };
}

/** A group by the name of a person, a label or a tag, as the viewer sees it. */
function named(id: string, name: string | undefined): GroupOfKey {
  return { ...none, id, filter: name, name };
}

/**
 * The group a key stands for and how its rows are asked for (`<field> is <value>`, or
 * `<field> is empty`). `choices` are those of the field the list is grouped by. A key that the
 * server gives as an ID or a stored key has no name the language knows, so it has no filter.
 */
export function groupOfKey(key: GroupKey, choices: readonly string[]): GroupOfKey {
  if (key.type === "empty") {
    return { ...none, id: "empty", filter: { empty: true } };
  }
  if (key.type === "priority") {
    return {
      ...none,
      id: `priority:${key.value}`,
      filter: choiceFor(key.value, choices),
      priority: key.value,
    };
  }
  if (key.type === "status-group") {
    return {
      ...none,
      id: `status-group:${key.value}`,
      filter: choiceFor(key.value, choices),
      statusGroup: key.value,
    };
  }
  if (key.type === "person") {
    const name = personName(key.value);
    return named(`person:${name ?? "-"}`, name);
  }
  if (key.type === "tag" || key.type === "label") {
    return named(`${key.type}:${key.value.id}`, key.value.name ?? undefined);
  }
  if (key.type === "text") {
    return { ...unnamed(key.type, key.value), name: key.value };
  }
  if (key.type === "whole") {
    return { ...unnamed(key.type, String(key.value)), name: String(key.value) };
  }
  return unnamed(key.type, key.value);
}
