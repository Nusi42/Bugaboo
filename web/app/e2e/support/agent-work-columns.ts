import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../../src/shared/paths.ts";
import { ada } from "./accounts.ts";
import { sceneIssue } from "./agent-work-ready.ts";
import { test } from "./fixtures.ts";
import { signIn } from "./flows.ts";
import { layoutOf } from "./layout.ts";
import { inOrder } from "./sequence.ts";
import type { Instance } from "./server.ts";

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
      await panel.getByText(name, { exact: true }).click();
    }),
  );
}

/** A phone has no Customize: the test that needs it is skipped there. */
export function skipOnAPhone(page: Page): void {
  test.skip(layoutOf(page) === "phone", "a phone has no Customize");
}
