import { expect, type Page } from "@playwright/test";

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
