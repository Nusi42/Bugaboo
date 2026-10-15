import { dataOf, type Schema } from "@bugaboo/api-client";
import { giveRole, makeGroup, roleName } from "./access.ts";
import { bob, makeAdmin, makePerson, ownId } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { makeProject } from "./projects.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

/** A field of the list by its name; the test stops with a plain message when it is not there. */
export function fieldNamed(
  fields: readonly Schema<"FieldBody">[],
  name: string,
): Schema<"FieldBody"> {
  const found = fields.find((each) => each.name === name);
  if (found === undefined) {
    throw new TypeError(`The organization has no field called ${name}.`);
  }
  return found;
}

/** A field of the organization by its name, read through the API. */
export async function fieldOf(session: ApiSession, name: string): Promise<Schema<"FieldBody">> {
  return fieldNamed(dataOf(await session.client.GET("/api/v1/fields")).fields, name);
}

/** Makes an own field for issues, through the API; the field as the server made it. */
export async function makeOwnField(
  admin: ApiSession,
  name: string,
  kind: Schema<"FieldType">,
  choices: readonly string[] = [],
): Promise<Schema<"FieldBody">> {
  return dataOf(
    await admin.client.POST("/api/v1/fields", {
      body: {
        name,
        kind,
        applies_to: "issues",
        choices: choices.map((each) => ({ name: each })),
      },
    }),
  );
}

/**
 * Gets a server ready for the tests of own fields on the issue page: the project WEB, a person
 * (Bob), and the fields Severity (pick one), Platforms (pick several), Customer (text) and Notes
 * (long text) for issues.
 */
export async function prepareFieldsIssue(server: Instance): Promise<void> {
  const admin = await makeAdmin(server);
  await (await makePerson(server, admin, bob)).dispose();
  await makeProject(admin, "WEB", "Website");
  await inOrder([
    () => makeOwnField(admin, "Severity", "pick-one", ["Minor", "Critical"]),
    () => makeOwnField(admin, "Platforms", "pick-several", ["Windows", "Mac"]),
    () => makeOwnField(admin, "Customer", "text"),
    () => makeOwnField(admin, "Notes", "long-text"),
  ]);
  await admin.dispose();
}

/** The name of the group the pickers can choose. */
export const pickableGroup = "Core team";

/**
 * Gets a server ready for the tests of the pickers: the project WEB, a Team member (Bob), a group, and
 * the fields Tester (person), Team (group), Duplicate of (issue) and Notes (long text).
 */
export async function preparePickers(server: Instance): Promise<void> {
  const admin = await makeAdmin(server);
  const person = await makePerson(server, admin, bob);
  await giveRole(admin, await ownId(person), roleName.teamMember, null);
  await person.dispose();
  await makeProject(admin, "WEB", "Website");
  await makeGroup(admin, pickableGroup);
  await inOrder([
    () => makeOwnField(admin, "Tester", "person"),
    () => makeOwnField(admin, "Team", "group"),
    () => makeOwnField(admin, "Duplicate of", "issue"),
    () => makeOwnField(admin, "Notes", "long-text"),
  ]);
  await admin.dispose();
}
