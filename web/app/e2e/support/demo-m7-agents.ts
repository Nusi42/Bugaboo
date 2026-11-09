import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { tool } from "../../../tooling/load/agents.ts";
import { type ApiSession } from "./api.ts";
import { good, adaOfTheSample } from "./demo-m7-common.ts";
import { signIn } from "./flows.ts";
import { openHandOver, openWindow } from "./agent-work.ts";
import { type Instance } from "./server.ts";

// The agent steps of the exit demo of M7 (`m7-exit-demo.spec.ts`): Ada hands issues to her two
// agents, one asks a question, one is sent back and accepted, one is thrown away, and a standing
// rule only proposes. The agents work through their tools over `/mcp`, as a real agent does: no
// row is put into the database by the test (M7-F108).

/** The issues of the demo: the one that asks, the one that is sent back, the one thrown away. */
export const ISSUES = { asks: "WEB-150", sentBack: "WEB-151", thrown: "WEB-152" } as const;

/** An agent of Ada: its ID and its key. */
export interface AdasAgent {
  readonly id: string;
  readonly key: string;
}

/** Ada makes an agent with a key, through the API, as she would on the page of an agent. */
export async function adasAgent(ada: ApiSession, name: string): Promise<AdasAgent> {
  const made = dataOf(await ada.client.POST("/api/v1/agents", { body: { name } }));
  const id = made.id ?? "";
  const key = dataOf(
    await ada.client.POST("/api/v1/agents/{agent}/keys", {
      params: { path: { agent: id } },
      body: { name: "Key", password: adaOfTheSample().password },
    }),
  );
  return { id, key: key.secret ?? "" };
}

/** The agent calls a tool and has to be answered. */
async function does(
  server: Instance,
  agent: AdasAgent,
  name: string,
  args: Readonly<Record<string, unknown>>,
): Promise<void> {
  expect((await tool(server, agent.key, name, args)).kept, `${name} of the agent`).toBe(true);
}

/** The agent takes the issue and reports on it. */
export async function startsWork(server: Instance, agent: AdasAgent, issue: string) {
  await does(server, agent, "take", { issue });
  await does(server, agent, "report", { issue, minutes: 4, cost_cents: 0 });
}

/** The agent ends its attempt with a result for the review. */
export async function submits(server: Instance, agent: AdasAgent, issue: string) {
  await does(server, agent, "submit", { issue });
}

/** The version of an issue and its priority, as Ada reads them. */
export async function priorityOf(ada: ApiSession, issue: string) {
  const found = dataOf(
    await ada.client.GET("/api/v1/issues/{issue}", { params: { path: { issue } } }),
  );
  return { priority: found.issue.priority ?? "", version: found.issue.version ?? 1 };
}

/** Ada hands the issue over to the agent in the window of the page, as a person does. */
export async function handsOverInTheWindow(page: Page, agent: string, issue: string) {
  await signIn(page, adaOfTheSample());
  await page.goto(issueAddress(issue));
  const window = await openHandOver(page);
  await window.getByLabel(t("agent-work.agent"), { exact: true }).selectOption({ label: agent });
  await good(page, "03-a-the-hand-over");
  await window
    .getByRole("button", { name: t("agent-work.hand-over-confirm"), exact: true })
    .click();
  await expect(window).toBeHidden();
}

/** Ada hands the issue over and lets the agent change the priority (through the API). */
export async function handsOverWithPriority(
  ada: ApiSession,
  agent: AdasAgent,
  issue: string,
): Promise<void> {
  dataOf(
    await ada.client.POST("/api/v1/hand-overs", {
      body: {
        agent: agent.id,
        issues: [issue],
        note: "Please look at this",
        powers: ["change-fields"],
        fields: ["priority"],
      },
    }),
  );
}

/** Ada answers the question of the agent on the issue page (on a phone when the test runs on one). */
export async function answers(page: Page, issue: string, question: string, answer: string) {
  await page.goto(issueAddress(issue));
  await expect(page.getByText(question, { exact: false })).toBeVisible();
  await good(page, "03-b-the-question");
  await page.getByLabel(t("agent-work.question-answer"), { exact: true }).fill(answer);
  await page.getByRole("button", { name: t("agent-work.question-send"), exact: true }).click();
  await expect(page.getByText(t("agent-work.answered-by", { person: "Ada Brandt" }))).toBeVisible();
  await good(page, "03-c-the-answer");
}

/** Ada reviews the result: opens the window, chooses what to do, and confirms. */
export async function reviews(
  page: Page,
  issue: string,
  choice: "send-back" | "throw-away" | "accept",
  note?: string,
) {
  await page.goto(issueAddress(issue));
  await page.getByRole("button", { name: t("agent-work.review"), exact: true }).click();
  const window = await openWindow(page);
  if (choice !== "accept") {
    await window.getByRole("radio", { name: t(`agent-work.review-${choice}-choice`) }).check();
  }
  if (note !== undefined) {
    await window.getByLabel(t("agent-work.review-note-label")).fill(note);
  }
  await good(page, `04-${choice}`);
  await window
    .getByRole("button", {
      name: t(
        choice === "accept"
          ? "agent-work.review-confirm-accept"
          : choice === "send-back"
            ? "agent-work.review-confirm-send-back"
            : "agent-work.review-confirm-throw-away",
      ),
      exact: true,
    })
    .click();
  await expect(window).toBeHidden();
}
