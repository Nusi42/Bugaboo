import { giveRole, roleName } from "../../app/e2e/support/access.ts";
import { ada, makeAdmin, makePerson, ownId, type Person } from "../../app/e2e/support/accounts.ts";
import { type ApiSession } from "../../app/e2e/support/api.ts";
import { countIssueIds, makeIssue } from "../../app/e2e/support/issues.ts";
import { makeProject } from "../../app/e2e/support/projects.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";
import { type Instance } from "../../app/e2e/support/server.ts";
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
  await countIssueIds(admin);
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
