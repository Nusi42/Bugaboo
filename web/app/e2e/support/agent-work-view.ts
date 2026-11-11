import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { part } from "../../../packages/ui/src/tokens.ts";
import { waitUntilStill } from "./agent-work-still.ts";
import { issueMoreOf } from "./layout.ts";
import { partIn } from "./parts.ts";
import { answerTo } from "./sequence.ts";

/**
 * Opens *More* of the issue and chooses *Hand over to an agent*; the window it opens. *More* is the
 * one of the issue: on a phone the bar of the app with its own *More* stands under the page until
 * the issue has loaded and its bar takes the place, and that *More* has no hand-over.
 */
export async function openHandOver(page: Page): Promise<Locator> {
  await issueMoreOf(page).click();
  await page.getByRole("button", { name: t("agent-work.hand-over"), exact: true }).click();
  const window = page.getByRole("dialog");
  await expect(window).toBeVisible();
  // It has slid in (on a phone) before anything is done in it.
  await waitUntilStill(window);
  return window;
}

/**
 * Brings the agent work of the issue page into view from its start, so its head (who works, and the
 * state of the work) shows whole, and the text under it that a step is about (a question) shows
 * too (M7-F245: the question in the middle of the screen pushed the state out under the bar).
 */
export async function showsTheAgentWork(page: Page, text: string): Promise<void> {
  await partIn(page, part.agentWork)
    .first()
    .evaluate((node: HTMLElement) => {
      node.scrollIntoView({ block: "start" });
    });
  await expect(page.getByText(text).first()).toBeInViewport();
}

/** The next read of the agent work of an issue that the page makes, as it does again after a refusal. */
export function agentWorkRead(page: Page, issue: string): Promise<unknown> {
  return answerTo(page, "GET", `/api/v1/issues/${issue}/agent-work`);
}
