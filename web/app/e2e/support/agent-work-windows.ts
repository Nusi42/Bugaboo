import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { issueAddress, projectIssuesAddress } from "../../src/shared/paths.ts";
import { ada } from "./accounts.ts";
import { signedInAdmin } from "./access.ts";
import { createAgent, openWindow } from "./agent-work.ts";
import { readyForReview, sceneIssue } from "./agent-work-ready.ts";
import { signIn } from "./flows.ts";
import { inOrder } from "./sequence.ts";
import type { Instance } from "./server.ts";

/** Creates an agent of the administrator and signs the administrator in on the page. */
export async function signedInWithAgent(page: Page, server: Instance, name: string): Promise<void> {
  await createAgent(await signedInAdmin(server), name);
  await signIn(page, ada);
}

/** Signs in and opens the review window of an attempt that is ready; the window. */
export async function openReview(page: Page, server: Instance): Promise<Locator> {
  const { issue } = await readyForReview(server, "Review pictures", "Reviewer", true);
  await signIn(page, ada);
  await page.goto(issueAddress(issue.key));
  await page.getByRole("button", { name: t("agent-work.review"), exact: true }).click();
  return openWindow(page);
}

/** Signs in, opens the list and chooses the columns Agent and Agent state in Customize. */
export async function showAgentColumns(page: Page, server: Instance): Promise<void> {
  const { issue } = await sceneIssue(server, "A row with the Agent column", "Columner", {
    state: "waiting-for-you",
    question: "Which one?",
  });
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.getByText(issue.key, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: t("issues.customize") });
  await inOrder(
    [t("issue-field.agent"), t("issue-field.agent-state")].map((name) => async () => {
      await panel.getByRole("checkbox", { name, exact: true }).check({ force: true });
    }),
  );
}
