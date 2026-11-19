import { dataOf } from "@bugaboo/api-client";

import { ada } from "./accounts.ts";
import { signedInAdmin } from "./access.ts";
import { expect } from "./fixtures.ts";
import { type Instance } from "./server.ts";
import { exchange } from "./session-transport.ts";

// Requests for agents (ADR-0029, amendment M9): a key of Ada, and a program that asks
// with it over `/mcp`, as a script would; no row is put into the database by the test.

/** A program with a key of Ada asks for agents over `/mcp`, as a script would. */
export async function aProgramAsks(server: Instance, key: string, agents: readonly string[]) {
  const answered = await exchange(
    {
      url: `${server.url}/mcp`,
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
        params: {
          name: "ask_for_agents",
          arguments: {
            reason: "A team for the release",
            agents: agents.map((name) => ({ name, purpose: "works" })),
          },
        },
      }),
    },
    server.url,
  );
  expect(answered.body).toContain("waiting");
}

/** A key of Ada that may change things, made through the API. */
export async function adasKey(server: Instance): Promise<string> {
  const admin = await signedInAdmin(server);
  const made = dataOf(
    await admin.client.POST("/api/v1/access-keys", {
      body: {
        name: `Script ${String(Date.now())}`,
        password: ada.password,
        choice: "see-and-change",
        permissions: [],
        projects: [],
      },
    }),
  );
  await admin.dispose();
  return made.secret;
}
