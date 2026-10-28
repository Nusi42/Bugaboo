import { readFileSync } from "node:fs";
import { join } from "node:path";

import layout from "../../../../tools/dev/sample.json" with { type: "json" };
import { sampleFolder, samplePeople, type SamplePerson } from "./sample.ts";

// The groups of the sample, as its dump lists them.

/** The addresses of the people in a group of the sample. */
function groupMembers(group: string): readonly string[] {
  const dump: unknown = JSON.parse(readFileSync(join(sampleFolder(), layout.dumpFile), "utf8"));
  const groups: unknown =
    typeof dump === "object" && dump !== null ? Reflect.get(dump, "groups") : undefined;
  if (!Array.isArray(groups)) {
    throw new TypeError(`The dump ${layout.dumpFile} has no groups.`);
  }
  const members: string[] = [];
  for (const each of groups as readonly unknown[]) {
    const listed: unknown =
      typeof each === "object" && each !== null && Reflect.get(each, "name") === group
        ? Reflect.get(each, "members")
        : undefined;
    if (Array.isArray(listed)) {
      for (const member of listed as readonly unknown[]) {
        if (typeof member === "string") {
          members.push(member);
        }
      }
    }
  }
  return members;
}

/** A person in the group (by its name) who signs in with the password alone (no code app). */
export function sampleInGroup(group: string): SamplePerson {
  const members = groupMembers(group);
  const found = samplePeople().find(
    (person) => person.codeAppSecret === undefined && members.includes(person.email),
  );
  if (found === undefined) {
    throw new Error(`The sample has nobody in the group ${group} without a code app.`);
  }
  return found;
}
