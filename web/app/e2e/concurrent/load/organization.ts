import { settingKeys } from "@bugaboo/api-client";

import { giveRole, roleName } from "../../support/access.ts";
import { ada, makeAdmin, makePerson, ownId, type Person } from "../../support/accounts.ts";
import { type ApiSession } from "../../support/api.ts";
import { makeIssue } from "../../support/issues.ts";
import { makeProject } from "../../support/projects.ts";
import { inOrder } from "../../support/sequence.ts";
import { type Instance } from "../../support/server.ts";
import { type Work } from "./book.ts";

/** How many issues the people share: few, so that they meet. */
const ISSUES = 6;

/** The people of a run: the administrator first, then Team members. */
function peopleOf(count: number): readonly Person[] {
  return [
    ada,
    ...Array.from({ length: count - 1 }, (_, at) => ({
      name: `Load Person ${String(at + 2)}`,
      email: `load${String(at + 2)}@example.test`,
      password: ada.password,
    })),
  ];
}

/** The organization of a run: one project, the issues, and the people with their sessions. */
export async function prepare(
  server: Instance,
  count: number,
): Promise<{
  readonly sessions: readonly ApiSession[];
  readonly ids: readonly string[];
  readonly issues: readonly Work[];
}> {
  const admin = await makeAdmin(server);
  await admin.client.PUT("/api/v1/settings/{key}", {
    params: { path: { key: settingKeys.issuesIdMode } },
    body: { value: "counting" },
  });
  await makeProject(admin, "LOAD", "Load");
  const issues: Work[] = [];
  await inOrder(
    Array.from({ length: ISSUES }, (_, at) => async () => {
      issues.push(await makeIssue(admin, "LOAD", `Shared issue ${String(at + 1)}`));
    }),
  );
  const sessions: ApiSession[] = [admin];
  await inOrder(
    peopleOf(count)
      .slice(1)
      .map((person) => async () => {
        const session = await makePerson(server, admin, person);
        await giveRole(admin, await ownId(session), roleName.teamMember, "LOAD");
        sessions.push(session);
      }),
  );
  return {
    sessions,
    ids: await Promise.all(sessions.map((session) => ownId(session))),
    issues,
  };
}
