import { dataOf, type Client } from "@bugaboo/api-client";

import { ada, ownId } from "../../app/e2e/support/accounts.ts";
import { type ApiSession } from "../../app/e2e/support/api.ts";
import { createIssue } from "../../app/e2e/support/issues.ts";
import { type Instance } from "../../app/e2e/support/server.ts";
import { exchange } from "../../app/e2e/support/session-transport.ts";
import { type BookWriter, type Work } from "./book.ts";

// The agents of the ten people of a load run (architecture §11, "Ten people with agents", AI-30 to
// AI-38): what one person and their agent do. `agents-run.ts` lets all of them work at once.

/** The project of the run. */
export const PROJECT = "LOAD";
const MCP = "/mcp";

/** A person with their agent, the key of the agent and the issue handed over to it. */
export interface Crew {
  readonly number: number;
  readonly holder: string;
  readonly client: Client;
  readonly agent: string;
  readonly key: string;
  readonly issue: Work;
}

/** What the server answered a tool of an agent: the key was let in and the tool did its work. */
export interface ToolAnswer {
  readonly kept: boolean;
}

/** The agent calls a tool over `/mcp` with its key. */
export async function tool(
  server: Instance,
  key: string,
  name: string,
  args: Readonly<Record<string, unknown>>,
): Promise<ToolAnswer> {
  // The server speaks HTTPS with its own certificate, so the request goes through the pool of the
  // tests like every other; an app sends no `Origin`.
  const answered = await exchange(
    {
      url: `${server.url}${MCP}`,
      method: "POST",
      headers: {
        "content-type": "application/json",
        accept: "application/json, text/event-stream",
        authorization: `Bearer ${key}`,
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "tools/call",
        params: { name, arguments: args },
      }),
    },
    server.url,
  );
  let body: unknown = null;
  try {
    body = JSON.parse(answered.body);
  } catch {
    // A refusal of the key is not a message of the protocol.
  }
  const result: unknown =
    typeof body === "object" && body !== null ? Reflect.get(body, "result") : undefined;
  const failed: unknown =
    typeof result === "object" && result !== null ? Reflect.get(result, "isError") : undefined;
  return { kept: answered.status === 200 && failed !== true };
}

/** The agent calls a tool that has to be answered. */
export async function mustTool(
  server: Instance,
  crew: Crew,
  name: string,
  args: Readonly<Record<string, unknown>>,
): Promise<void> {
  if (!(await tool(server, crew.key, name, args)).kept) {
    throw new Error(`${name} of the agent of person ${String(crew.number)} was refused`);
  }
}

/** The version of the issue, as the person reads it. */
export async function versionOf(crew: Crew): Promise<number> {
  const found = dataOf(
    await crew.client.GET("/api/v1/issues/{issue}", {
      params: { path: { issue: crew.issue.key } },
    }),
  );
  return found.issue.version ?? 0;
}

/** The work on the issue of the crew: its ID, version and first question. */
async function workOf(crew: Crew): Promise<{
  readonly id: string;
  readonly version: number;
  readonly question: string;
}> {
  const found = dataOf(
    await crew.client.GET("/api/v1/issues/{issue}/agent-work", {
      params: { path: { issue: crew.issue.key } },
    }),
  );
  const piece = found.work[0];
  if (piece?.id === undefined || piece.id === null) {
    throw new Error(`${crew.issue.key} has no agent work`);
  }
  return {
    id: piece.id,
    version: piece.version ?? 0,
    question: piece.questions?.[0]?.id ?? "",
  };
}

/** The person answers the result of the work. */
export async function review(
  crew: Crew,
  answer: "accept" | "send-back" | "throw-away",
  note: string | null,
): Promise<void> {
  const piece = await workOf(crew);
  dataOf(
    await crew.client.POST("/api/v1/agent-work/{work}/review", {
      params: { path: { work: piece.id } },
      body: { answer, note, version: piece.version },
    }),
  );
}

/** A person with an agent that has a key, and an issue made for it. */
export async function newCrew(
  admin: ApiSession,
  session: ApiSession,
  number: number,
): Promise<Crew> {
  const made = dataOf(
    await session.client.POST("/api/v1/agents", { body: { name: `Agent ${String(number)}` } }),
  );
  const agent = made.id ?? "";
  const key = dataOf(
    await session.client.POST("/api/v1/agents/{agent}/keys", {
      params: { path: { agent } },
      body: { name: "Key", password: ada.password },
    }),
  );
  return {
    number,
    holder: await ownId(session),
    client: session.client,
    agent,
    key: key.secret ?? "",
    issue: await createIssue(admin, PROJECT, `Issue of agent ${String(number)}`),
  };
}

/** The person hands the issue over to their agent, with the right to change the priority. */
export async function handOver(crew: Crew, issue: Work, book: BookWriter): Promise<void> {
  dataOf(
    await crew.client.POST("/api/v1/hand-overs", {
      body: {
        agent: crew.agent,
        issues: [issue.key],
        note: "Please look at this",
        powers: ["change-fields"],
        fields: ["priority"],
      },
    }),
  );
  book.handed({ agent: crew.agent, key: issue.key });
}

/** One attempt: take, report, change the priority, perhaps ask, and submit. */
export async function attempt(
  server: Instance,
  crew: Crew,
  book: BookWriter,
  options: { readonly asks: boolean; readonly changes: boolean },
): Promise<void> {
  const issue = crew.issue.key;
  await mustTool(server, crew, "take", { issue });
  await mustTool(server, crew, "report", { issue, minutes: 3, cost_cents: 0 });
  if (options.changes) {
    const version = await versionOf(crew);
    await mustTool(server, crew, "update_issue", { issue, version, priority: "high" });
    book.kept({ key: issue, version: version + 1 });
  }
  if (options.asks) {
    await mustTool(server, crew, "ask", { issue, question: "Which browser?" });
    const piece = await workOf(crew);
    dataOf(
      await crew.client.POST("/api/v1/agent-work/{work}/answer", {
        params: { path: { work: piece.id } },
        body: { question: piece.question, answer: "Firefox", version: piece.version },
      }),
    );
  }
  await mustTool(server, crew, "submit", { issue });
  // Ending an attempt writes the short comment of the agent on the issue.
  book.comment(issue);
}
