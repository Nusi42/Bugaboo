import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** A person as the server shows them to the person who asks (`PersonRef`). */
export type Person = Schema<"PersonRef">;

/** A person who is shown by name, which is the only kind that has an ID to act on. */
export interface NamedPerson {
  readonly id: string;
  readonly name: string;
}

/**
 * The person as an ID and a name, or nothing when they are not shown by name (by their role, as
 * *Someone*, or as a former person): nobody else can be chosen, listed or acted on. A missing
 * person (the field was not asked for) is nothing too.
 */
export function namedPerson(person: Person | null | undefined): NamedPerson | undefined {
  return person?.shown === "name" ? { id: person.id, name: person.name } : undefined;
}

/** Whom a row of *Who has which role* or a pick is for: a person, or a group of people. */
export type SubjectKind = "person" | "group";

/** Where in a sentence a person is named: a role reads differently at the start and in the middle. */
export type Position = "start" | "inside";

const roleMessage = {
  start: "projects.person-role",
  inside: "access.person-role-inside",
} as const;
const formerMessage = {
  start: "projects.person-former",
  inside: "access.person-former-inside",
} as const;
const someoneMessage = {
  start: "projects.person-someone",
  inside: "access.person-someone-inside",
} as const;

/** What a person is called when they are shown by their role only: *a Team member*. */
export function roleName(position: Position, role: string): string {
  return t(roleMessage[position], { role });
}

/** What a person is called when nothing of them is shown: *Someone*. */
export function someoneName(position: Position): string {
  return t(someoneMessage[position]);
}

/** How a person is named at a place in a sentence: by name, by their role, or as *Someone*. */
export function personNameAt(position: Position, person: Person): string {
  if (person.shown === "name") {
    return person.name;
  }
  if (person.shown === "role") {
    return roleName(position, person.role);
  }
  if (person.shown === "former") {
    return t(formerMessage[position], { label: person.label });
  }
  return someoneName(position);
}

/** How a person is named at the start of a sentence: by name, by their role, or as *Someone*. */
export function personName(person: Person): string {
  return personNameAt("start", person);
}

/** How a person is named in the middle of a sentence: a name stays, a role is "a Team member". */
export function personNameInside(person: Person): string {
  return personNameAt("inside", person);
}
