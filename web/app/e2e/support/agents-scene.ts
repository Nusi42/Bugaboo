import { dataOf } from "@bugaboo/api-client";

import { signedInAdmin } from "./access.ts";
import { type ApiSession } from "./api.ts";
import { webIssues } from "./issues.ts";

/** The names of the agents of the scene. */
export const agentNames = { fixer: "Fixer", sorter: "Sorter" } as const;

/** The saved search of the standing rule of the scene. */
export const ruleSearchName = "New bugs in Website";

/** What a scene of agents holds, by ID. */
export interface AgentsScene {
  readonly admin: ApiSession;
  readonly fixer: string;
  readonly sorter: string;
  readonly search: string;
  readonly project: string;
}

/** Creates an agent for the person of the session; its ID. */
export async function createAgent(session: ApiSession, name: string): Promise<string> {
  const made = dataOf(await session.client.POST("/api/v1/agents", { body: { name } }));
  if (made.id === undefined || made.id === null) {
    throw new TypeError("The server made an agent without an ID.");
  }
  return made.id;
}

/** Creates a standing rule of an agent that runs out after `days`. */
export async function createRule(
  session: ApiSession,
  scene: Pick<AgentsScene, "project" | "search">,
  agent: string,
  days: number,
): Promise<void> {
  dataOf(
    await session.client.POST("/api/v1/standing-rules", {
      body: { agent, search: scene.search, project: scene.project, fields: [], days },
    }),
  );
}

/**
 * Two agents of the administrator: Fixer, handed the first issue of the project WEB, and Sorter, with a
 * standing rule over a saved search that runs out in two days. The server is the one `prepareWeb` made.
 */
export async function agentsScene(
  server: Parameters<typeof signedInAdmin>[0],
): Promise<AgentsScene> {
  const admin = await signedInAdmin(server);
  const project = dataOf(
    await admin.client.GET("/api/v1/projects/{project}", { params: { path: { project: "WEB" } } }),
  ).project.id;
  const search = dataOf(
    await admin.client.POST("/api/v1/saved-searches", {
      body: { name: ruleSearchName, search_text: "" },
    }),
  ).id;
  const fixer = await createAgent(admin, agentNames.fixer);
  const sorter = await createAgent(admin, agentNames.sorter);
  dataOf(
    await admin.client.POST("/api/v1/hand-overs", {
      body: {
        agent: fixer,
        issues: ["WEB-1"],
        note: `Please look at: ${webIssues.login}`,
        powers: [],
        fields: [],
        plan_needs_approval: false,
        accept_by_itself: false,
      },
    }),
  );
  const scene = { admin, fixer, sorter, search, project };
  await createRule(admin, scene, sorter, 2);
  return scene;
}

/** Creates agents for the person of the session one after the other, so their order is the order they were made in. */
export async function createAgentsInOrder(
  session: ApiSession,
  names: readonly string[],
): Promise<void> {
  const [first, ...rest] = names;
  if (first !== undefined) {
    await createAgent(session, first);
    await createAgentsInOrder(session, rest);
  }
}
