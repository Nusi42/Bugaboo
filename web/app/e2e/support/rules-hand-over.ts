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
  await window
    .getByLabel(t("agents.rule-agent"), { exact: true })
    .selectOption({ label: agentName });
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
        accept_by_itself: true,
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

/**
 * Scrolls the body of the window, never the page behind it, so that the text is at the top of what
 * the window shows (the window keeps its place and the layer behind it covers the page).
 */
export async function scrollInWindow(page: Page, text: string): Promise<void> {
  await pageBackToTop(page);
  await page
    .getByRole("alertdialog")
    .getByText(text, { exact: true })
    .evaluate((node: HTMLElement) => {
      const window = node.closest("[role='alertdialog']");
      let scroller: HTMLElement | null = node.parentElement;
      while (
        scroller !== null &&
        scroller !== window &&
        scroller.scrollHeight <= scroller.clientHeight
      ) {
        scroller = scroller.parentElement;
      }
      if (scroller !== null && scroller !== window) {
        scroller.scrollTop +=
          node.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
      }
    });
}

/** Puts the page behind the window back to its top, as a person who has not scrolled it sees it. */
export async function pageBackToTop(page: Page): Promise<void> {
  await page.evaluate(() => {
    const window = document.querySelector("[role='alertdialog']");
    for (const element of document.querySelectorAll("*")) {
      if (element.scrollTop > 0 && window?.contains(element) !== true) {
        element.scrollTop = 0;
      }
    }
  });
}

/** Shows the way section at the top of the window: the choice, its help and what the agent may do. */
export async function scrollToWay(page: Page): Promise<void> {
  await pageBackToTop(page);
  await scrollInWindow(page, t("agents.rule-may"));
}
