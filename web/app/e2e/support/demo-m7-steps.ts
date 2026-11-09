import { dataOf } from "@bugaboo/api-client";
import { expect, type Page } from "@playwright/test";

import { tool } from "../../../tooling/load/agents.ts";
import { signInThroughApi } from "./accounts.ts";
import {
  ISSUES,
  adasAgent,
  answers,
  handsOverInTheWindow,
  handsOverWithPriority,
  priorityOf,
  reviews,
  startsWork,
  submits,
} from "./demo-m7-agents.ts";
import { adaOfTheSample } from "./demo-m7-common.ts";
import { signIn } from "./flows.ts";
import { type Instance } from "./server.ts";

// Steps 3 to 5 of the exit demo of M7 (`m7-exit-demo.spec.ts`): Ada's agents and a standing rule.

const QUESTION = "Which browser shows the problem?";

/** The state of the agent work on an issue, as Ada reads it. */
async function workStateOf(server: Instance, issue: string) {
  const ada = await signInThroughApi(server, adaOfTheSample());
  const found = dataOf(
    await ada.client.GET("/api/v1/issues/{issue}/agent-work", { params: { path: { issue } } }),
  );
  await ada.dispose();
  return { state: found.work[0]?.state, open: found.work[0]?.open };
}

/** Step 3: one agent asks a question that Ada answers; the answer joins the brief. */
export async function anAgentAsksAndAdaAnswers(page: Page, server: Instance) {
  const ada = await signInThroughApi(server, adaOfTheSample());
  const fixer = await adasAgent(ada, "Fixer");
  await handsOverInTheWindow(page, "Fixer", ISSUES.asks);
  await startsWork(server, fixer, ISSUES.asks);
  const asked = await tool(server, fixer.key, "ask", { issue: ISSUES.asks, question: QUESTION });
  expect(asked.kept).toBe(true);
  await answers(page, ISSUES.asks, QUESTION, "Firefox on a phone");
  const work = dataOf(
    await ada.client.GET("/api/v1/issues/{issue}/agent-work", {
      params: { path: { issue: ISSUES.asks } },
    }),
  );
  expect(work.work[0]?.brief.join(" ")).toContain("Firefox on a phone");
  await ada.dispose();
}

/** Step 4: the other agent is sent back once and then accepted. */
export async function anAgentIsSentBackOnceAndThenAccepted(page: Page, server: Instance) {
  const ada = await signInThroughApi(server, adaOfTheSample());
  await signIn(page, adaOfTheSample());
  const sorter = await adasAgent(ada, "Sorter");
  dataOf(
    await ada.client.POST("/api/v1/hand-overs", {
      body: { agent: sorter.id, issues: [ISSUES.sentBack], note: "Please look at this" },
    }),
  );
  await startsWork(server, sorter, ISSUES.sentBack);
  await submits(server, sorter, ISSUES.sentBack);
  await reviews(page, ISSUES.sentBack, "send-back", "Please also check the narrow layout");
  await expect
    .poll(async () => (await workStateOf(server, ISSUES.sentBack)).state)
    .toBe("waiting-for-the-agent");
  await startsWork(server, sorter, ISSUES.sentBack);
  await submits(server, sorter, ISSUES.sentBack);
  await reviews(page, ISSUES.sentBack, "accept");
  await expect.poll(async () => (await workStateOf(server, ISSUES.sentBack)).open).toBe(false);
  await ada.dispose();
}

/** Step 5: a third attempt is thrown away and its changes are undone in one step. */
export async function aThirdAttemptIsThrownAway(page: Page, server: Instance) {
  const ada = await signInThroughApi(server, adaOfTheSample());
  await signIn(page, adaOfTheSample());
  const helper = await adasAgent(ada, "Helper");
  const before = await priorityOf(ada, ISSUES.thrown);
  await handsOverWithPriority(ada, helper, ISSUES.thrown);
  await startsWork(server, helper, ISSUES.thrown);
  const changed = await tool(server, helper.key, "update_issue", {
    issue: ISSUES.thrown,
    version: before.version,
    priority: before.priority === "critical" ? "low" : "critical",
  });
  expect(changed.kept).toBe(true);
  await submits(server, helper, ISSUES.thrown);
  expect((await priorityOf(ada, ISSUES.thrown)).priority).not.toBe(before.priority);
  await reviews(page, ISSUES.thrown, "throw-away");
  await expect
    .poll(async () => (await priorityOf(ada, ISSUES.thrown)).priority, { timeout: 20_000 })
    .toBe(before.priority);
  await ada.dispose();
}
