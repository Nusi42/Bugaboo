import { dataOf } from "@bugaboo/api-client";

import { giveRole, projectIdOf, roleName } from "../../app/e2e/support/access.ts";
import { ada, createPerson, ownId } from "../../app/e2e/support/accounts.ts";
import { type ApiSession } from "../../app/e2e/support/api.ts";
import { switchAiOn } from "../../app/e2e/support/connect-app.ts";
import { createIssue } from "../../app/e2e/support/issues.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";
import { type Instance } from "../../app/e2e/support/server.ts";
import {
  PROJECT,
  type Crew,
  attempt,
  handOver,
  mustTool,
  newCrew,
  review,
  tool,
  versionOf,
} from "./agents.ts";
import { type BookWriter, type Work } from "./book.ts";

// Ten people and their agents work at the same moments: some reviews accept, some send an attempt
// back first, one throws an attempt away (which undoes the change of the agent), two agents take
// one issue at the same moment, and an agent goes on after its person lost access. The rules are
// judged by `bb-load-check` (crates/bb-load-check/src/agents.rs).

/** One person and their agent: how the review goes depends on the number of the person. */
async function workThrough(server: Instance, crew: Crew, book: BookWriter): Promise<void> {
  await handOver(crew, crew.issue, book);
  const before = await versionOf(crew);
  await attempt(server, crew, book, { asks: crew.number % 2 === 0, changes: true });
  switch (crew.number % 3) {
    case 0:
      await review(crew, "accept", null);
      break;
    case 1:
      await review(crew, "send-back", "Please also check the narrow layout");
      await attempt(server, crew, book, { asks: false, changes: false });
      await review(crew, "accept", null);
      break;
    default: {
      await review(crew, "throw-away", null);
      // The undo wrote the old value back: one more version, kept by the person.
      const now = await versionOf(crew);
      for (let version = before + 2; version <= now; version += 1) {
        book.kept({ key: crew.issue.key, version });
      }
    }
  }
}

/** Two agents take the same issue at the same moment, and one is told it has it. */
async function race(
  server: Instance,
  shared: Work,
  pair: readonly [Crew, Crew],
  book: BookWriter,
): Promise<void> {
  await Promise.all(pair.map((crew) => handOver(crew, shared, book)));
  const [one, other] = await Promise.all(
    pair.map((crew) => tool(server, crew.key, "take", { issue: shared.key })),
  );
  book.raced({
    key: shared.key,
    askers: pair.map((crew) => crew.agent),
    winners: pair.filter((_, at) => [one, other][at]?.kept === true).map((crew) => crew.agent),
  });
}

/** Takes the role of the person away on the project, as the administrator does. */
async function loseAccess(admin: ApiSession, person: string): Promise<void> {
  const rows = dataOf(
    await admin.client.GET("/api/v1/projects/{project}/assignments", {
      params: { path: { project: await projectIdOf(admin, PROJECT) } },
    }),
  );
  const row = rows.items.find((each) => {
    const who = each.person;
    return who !== null && who !== undefined && "id" in who && who.id === person;
  });
  if (row?.id === undefined || row.id === null) {
    throw new Error("the person has no role to lose");
  }
  await admin.client.DELETE("/api/v1/assignments/{assignment}", {
    params: { path: { assignment: row.id } },
  });
}

/** An agent works, its person loses access, and the agent's next calls have to be refused. */
async function workUntilAccessIsLost(
  server: Instance,
  admin: ApiSession,
  crew: Crew,
  book: BookWriter,
): Promise<void> {
  await handOver(crew, crew.issue, book);
  const issue = crew.issue.key;
  await mustTool(server, crew, "take", { issue });
  await mustTool(server, crew, "report", { issue, minutes: 1, cost_cents: 0 });
  await loseAccess(admin, crew.holder);
  const after = await Promise.all([
    tool(server, crew.key, "report", { issue, minutes: 2, cost_cents: 0 }),
    tool(server, crew.key, "submit", { issue }),
  ]);
  book.lost({
    agent: crew.agent,
    person: crew.holder,
    kept_after: after.filter((answer) => answer.kept).length,
  });
}

/** The people with agents of a run, made before the others start: the issues are all there. */
export interface Agents {
  readonly crews: readonly Crew[];
  /** The person who loses access, with their agent. */
  readonly lost: Crew;
  /** The person who loses access, as a session. */
  readonly leaver: ApiSession;
  /** The issue two agents take at the same moment. */
  readonly shared: Work;
  /** Every issue made for the agents. */
  readonly issues: readonly Work[];
}

/**
 * Makes the agents of the Team members of the run, the person who loses access and the issues, and
 * writes the agents into the book. It is done before the people start: a search for everything
 * judges the issues of the run, which must all be there by then.
 */
export async function prepareAgents(
  server: Instance,
  admin: ApiSession,
  team: readonly ApiSession[],
  book: BookWriter,
): Promise<Agents> {
  await switchAiOn(server);
  const crews: Crew[] = [];
  await inOrder(
    team.map((session, at) => async () => {
      crews.push(await newCrew(admin, session, at + 1));
    }),
  );
  const leaver = await createPerson(server, admin, {
    name: "Load Leaver",
    email: "leaver@example.test",
    password: ada.password,
  });
  await giveRole(admin, await ownId(leaver), roleName.teamMember, PROJECT);
  const lost = await newCrew(admin, leaver, crews.length + 1);
  for (const crew of [...crews, lost]) {
    book.agent({ agent: crew.agent, person: crew.holder });
  }
  const shared = await createIssue(admin, PROJECT, "Taken by two");
  return {
    crews,
    lost,
    leaver,
    shared,
    issues: [shared, ...[...crews, lost].map((crew) => crew.issue)],
  };
}

/**
 * Lets the agents work, two of them take one issue at the same moment, and an agent goes on after
 * its person lost access, all at the same moments.
 */
export async function workWithAgents(
  server: Instance,
  admin: ApiSession,
  agents: Agents,
  book: BookWriter,
): Promise<void> {
  const [first, second] = agents.crews;
  if (first === undefined || second === undefined) {
    throw new TypeError("A run with agents needs two people with agents.");
  }
  await Promise.all([
    race(server, agents.shared, [first, second], book),
    ...agents.crews.map((crew) => workThrough(server, crew, book)),
    workUntilAccessIsLost(server, admin, agents.lost, book),
  ]);
  await agents.leaver.dispose();
}
