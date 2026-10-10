import { dataOf, type Schema } from "@bugaboo/api-client";

import { bob, makeAdmin, makePerson } from "./accounts.ts";
import { makeProject } from "./projects.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

/**
 * Gets a server ready for the tests of own fields on the issue page: the project WEB, a person
 * (Bob), and the fields Severity (pick one), Platforms (pick several), Customer (text) and Notes
 * (long text) for issues.
 */
export async function prepareFieldsIssue(server: Instance): Promise<void> {
  const admin = await makeAdmin(server);
  await (await makePerson(server, admin, bob)).dispose();
  await makeProject(admin, "WEB", "Website");
  const make = async (name: string, kind: Schema<"FieldType">, choices: readonly string[] = []) => {
    dataOf(
      await admin.client.POST("/api/v1/fields", {
        body: {
          name,
          kind,
          applies_to: "issues",
          choices: choices.map((each) => ({ name: each })),
        },
      }),
    );
  };
  await inOrder([
    () => make("Severity", "pick-one", ["Minor", "Critical"]),
    () => make("Platforms", "pick-several", ["Windows", "Mac"]),
    () => make("Customer", "text"),
    () => make("Notes", "long-text"),
  ]);
  await admin.dispose();
}
