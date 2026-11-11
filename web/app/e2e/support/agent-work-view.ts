import { expect, type Page } from "@playwright/test";

import { answerTo } from "./sequence.ts";

/**
 * Brings the agent work of the issue page into view from its start, so its head (who works, and the
 * state of the work) shows whole, and the text under it that a step is about (a question) shows
 * too (M7-F245: the question in the middle of the screen pushed the state out under the bar).
 */
export async function showsTheAgentWork(page: Page, text: string): Promise<void> {
  await page.locator('[data-part="agent-work"]').evaluate((node: HTMLElement) => {
    node.scrollIntoView({ block: "start" });
  });
  await expect(page.getByText(text).first()).toBeInViewport();
}

/** The next read of the agent work of an issue that the page makes, as it does again after a refusal. */
export function agentWorkRead(page: Page, issue: string): Promise<unknown> {
  return answerTo(page, "GET", `/api/v1/issues/${issue}/agent-work`);
}
