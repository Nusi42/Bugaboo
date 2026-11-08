import { expect, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../../src/shared/paths.ts";
import { ada } from "./accounts.ts";
import { signedInAdmin } from "./access.ts";
import { createAgent, handOverOne, putScene } from "./agent-work.ts";
import { signIn } from "./flows.ts";
import { createIssue, tickBox } from "./issues.ts";
import { changeStatus } from "./notifications.ts";
import { inOrder } from "./sequence.ts";
import type { Instance } from "./server.ts";

/** Three issues of the three status groups, each with an agent at work, the keys in that order. */
export async function rowsOfStatuses(server: Instance): Promise<readonly string[]> {
  const admin = await signedInAdmin(server);
  const keys: string[] = [];
  await inOrder(
    (
      [
        ["Row not started", "open"],
        ["Row in progress", "in-progress"],
        ["Row done", "done"],
      ] as const
    ).map(([title, status]) => async () => {
      const issue = await createIssue(admin, "WEB", title);
      const agent = await createAgent(admin, `Agent of ${title}`);
      putScene(server, await handOverOne(admin, agent, issue), { state: "agent-working" });
      if (status !== "open") {
        await changeStatus(admin, issue.key, status);
      }
      keys.push(issue.key);
    }),
  );
  return keys;
}

/** Signs in, opens the list of the project with the rows of the three status groups and ticks the middle one; the first key. */
export async function openRowsList(page: Page, server: Instance): Promise<string> {
  const [first, second, third] = await rowsOfStatuses(server);
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.getByText(third ?? "", { exact: true })).toBeVisible();
  await tickBox(page, second ?? "").check({ force: true });
  return first ?? "";
}
