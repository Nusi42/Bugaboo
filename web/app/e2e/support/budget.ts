import { dataOf, type Schema } from "@bugaboo/api-client";

import { createAdmin } from "./accounts.ts";
import { createIssue } from "./issues.ts";
import { createProject } from "./projects.ts";
import type { Instance } from "./server.ts";

/** The choices of the field *Budget* of issues: a number and a name each, one with no number. */
const budgets = [
  { name: "Sales", number: "5200" },
  { name: "Product development", number: "4100" },
  { name: "Overhead", number: undefined },
] as const;

/**
 * A server with the project WEB, a field *Budget* of issues whose choices have numbers, and an
 * issue for each choice: what the search by a number starts from.
 */
export async function prepareBudgets(server: Instance): Promise<void> {
  const admin = await createAdmin(server);
  await createProject(admin, "WEB", "Website");
  const made: Schema<"FieldBody"> = dataOf(
    await admin.client.POST("/api/v1/fields", {
      body: {
        name: "Budget",
        kind: "pick-one",
        applies_to: "issues",
        choices: budgets.map(({ name, number }) => ({ name, number: number ?? null })),
      },
    }),
  );
  await Promise.all(
    made.choices.map((choice) =>
      createIssue(admin, "WEB", `Spent on ${choice.name}`, undefined, { [made.id]: choice.id }),
    ),
  );
  await admin.dispose();
}
