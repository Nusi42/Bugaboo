import { t } from "@bugaboo/messages";
import type { Locator, Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { ada } from "./accounts.ts";
import { signedInAdmin } from "./access.ts";
import { createAgent, openWindow } from "./agent-work.ts";
import { readyForReview } from "./agent-work-ready.ts";
import { signIn } from "./flows.ts";
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
