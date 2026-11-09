import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { appAtTheDoor, catchTheAnswer, codeOf } from "./connect-app.ts";
import { anaOfTheSample, good } from "./demo-m7-common.ts";
import { signIn } from "./flows.ts";
import { type Instance } from "./server.ts";
import { exchange } from "./session-transport.ts";

// The first two steps of the exit demo of M7 on the sample organization: an AI app of a Contributor
// and the assistant (in `demo-m7-assistant.ts`). The agent steps are in `demo-m7-agents.ts`.

const ISSUE = "WEB-142";

/** The text an app is given for a tool of `/mcp`, with its token. */
async function toolText(server: Instance, token: string, name: string, args: object) {
  const answered = await exchange(
    {
      url: `${server.url}/mcp`,
      method: "POST",
      headers: {
        "content-type": "application/json",
        accept: "application/json, text/event-stream",
        authorization: `Bearer ${token}`,
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
  return answered.body;
}

/** Step 1: an MCP client connects as a Contributor and does not see what is hidden from Contributors. */
export async function aContributorsAppSeesLessThanTheTeam(page: Page, server: Instance) {
  const door = await appAtTheDoor(server, "Claude Code", "see-only");
  await signIn(page, anaOfTheSample());
  await page.goto(door.address);
  await good(page, "01-a-the-app-asks");
  const answer = await catchTheAnswer(page, async () => {
    await page.getByRole("button", { name: t("connect.allow"), exact: true }).click();
  });
  const tokens = await door.exchange(codeOf(answer.toString()));
  const seen = await toolText(server, tokens.access, "get_issue", { issue: ISSUE });
  expect(seen).toContain(ISSUE);
  // The sample hides Customer, and the time fields Cost center, from Contributors.
  expect(seen).not.toContain("Customer");
  expect(seen).not.toContain("Cost center");
  await good(page, "01-b-the-app-is-back");
}
