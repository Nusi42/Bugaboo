import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** A person as the server shows them to the person who asks (`PersonRef`). */
export type Person = Schema<"PersonRef">;

/** How a person is named at the start of a sentence: by name, by their role, or as *Someone*. */
export function personName(person: Person): string {
  if (person.shown === "name") {
    return person.name;
  }
  if (person.shown === "role") {
    return t("projects.person-role", { role: person.role });
  }
  if (person.shown === "former") {
    return t("projects.person-former", { label: person.label });
  }
  return t("projects.person-someone");
}

/** How a person is named in the middle of a sentence: a name stays, "A Team member" becomes "a Team member". */
export function personNameInside(person: Person): string {
  const name = personName(person);
  return person.shown === "name" ? name : `${name.charAt(0).toLowerCase()}${name.slice(1)}`;
}
