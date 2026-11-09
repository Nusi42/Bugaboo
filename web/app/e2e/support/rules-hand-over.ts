import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { type AgentsScene, ruleSearchName } from "./agents-scene.ts";
import { type ApiSession } from "./api.ts";

// What the specs of the rules that hand over share (WP-14).

const NEVER_KEY = "agents.rules-never-run-out";

/** Lets rules never run out (or not) for the organization, as an administrator would in the settings. */
export async function allowNever(admin: ApiSession, on: boolean): Promise<void> {
  await admin.client.PUT("/api/v1/settings/{key}", {
    params: { path: { key: NEVER_KEY } },
    body: { value: on },
  });
}

/** Opens the window of a standing rule and fills it up to the way: the search, the agent, the project. */
export async function fillTopOfRule(page: Page, agentName: string): Promise<void> {
  await page.goto(paths.myAgents);
  await page.getByRole("button", { name: t("agents.rule-new"), exact: true }).click();
  const window = page.getByRole("alertdialog");
  await window.getByLabel(t("agents.rule-search")).selectOption({ label: ruleSearchName });
  await window.getByLabel(t("agents.rule-agent")).selectOption({ label: agentName });
  await window.getByLabel(t("agents.rule-project")).selectOption({ index: 1 });
}

/** How many pieces of work of the administrator the agent has now. */
export async function pieceCount(admin: ApiSession, agent: string): Promise<number> {
  const mine = dataOf(await admin.client.GET("/api/v1/me/agents/work"));
  return mine.pieces.filter(
    (piece: Readonly<{ work: Readonly<{ agent: string }> }>) => piece.work.agent === agent,
  ).length;
}

/** A rule of the agent that hands over what it finds and never runs out (the setting must allow it). */
export async function handingRuleWithNoEnd(scene: AgentsScene, agent: string): Promise<void> {
  dataOf(
    await scene.admin.client.POST("/api/v1/standing-rules", {
      body: {
        agent,
        search: scene.search,
        project: scene.project,
        way: "hand-over",
        powers: ["change-fields"],
        fields: ["title"],
        never_ends: true,
      },
    }),
  );
}

/** Chooses *Hand over what it finds* and ticks one field the agent may change, so the window can be sent. */
export async function handOverWithOneField(page: Page): Promise<void> {
  const window = page.getByRole("alertdialog");
  await window.getByLabel(t("agents.rule-hand-over")).check();
  await window.getByText(t("agent-work.power-change-fields"), { exact: true }).click();
  await window.getByText(t("issue-field.title"), { exact: true }).click();
}

/** Shows the way section at the top of the window: the choice, its help and what the agent may do. */
export async function scrollToWay(page: Page): Promise<void> {
  await page
    .getByRole("alertdialog")
    .getByText(t("agents.rule-may"), { exact: true })
    .evaluate((node: HTMLElement) => {
      node.scrollIntoView({ block: "start" });
    });
}
