import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { tool } from "../../../tooling/load/agents.ts";
import { signInThroughApi } from "./accounts.ts";
import { adaOfTheSample, benOfTheSample, good } from "./demo-m7-common.ts";
import { signIn } from "./flows.ts";
import { sampleAgents } from "./sample-agents.ts";
import { type Instance } from "./server.ts";

// Step 6 of the exit demo of M7 (`m7-exit-demo.spec.ts`): the standing rule of the sample.

/** The tag the rule proposes: a word a person would use for it. */
const TAG = "needs-triage";

/** Step 6: a standing rule over new bugs proposes and changes nothing until Ada presses Apply. */
export async function aStandingRuleProposesAndChangesNothing(page: Page, server: Instance) {
  const ben = await signInThroughApi(server, benOfTheSample());
  const agents = dataOf(await ben.client.GET("/api/v1/agents"));
  const sorter = agents.items.find((each) => each.name === "Sorter");
  const key = sampleAgents().find((each) => each.name === "Sorter")?.key ?? "";
  const rules = dataOf(
    await ben.client.GET("/api/v1/agents/{agent}/standing-rules", {
      params: { path: { agent: sorter?.id ?? "" } },
    }),
  );
  const rule = rules.items[0]?.id ?? "";
  const ada = await signInThroughApi(server, adaOfTheSample());
  const found = dataOf(
    await ada.client.GET("/api/v1/search/issues", {
      params: {
        query: { q: "type is Bug and statusGroup is not Done and project is WEB", limit: 1 },
      },
    }),
  );
  const issue = String(found.items[0]?.readable_id);
  const proposed = await tool(server, key, "propose", {
    rule,
    issue,
    field: "tags",
    value: TAG,
  });
  expect(proposed.kept).toBe(true);
  await signIn(page, adaOfTheSample());
  await page.goto(issueAddress(issue));
  // The proposal is the card of the assistant's proposals, and it says which agent it comes from.
  const card = page.getByRole("region", { name: t("assistant.card.title") });
  await expect(card).toBeVisible();
  await expect(card.getByText(TAG)).toBeVisible();
  await expect(card.getByText(t("agent-work.proposal-from", { agent: "Sorter" }))).toBeVisible();
  await expect(card.getByRole("button", { name: t("assistant.card.apply") })).toBeVisible();
  await card.evaluate((node: HTMLElement) => {
    node.scrollIntoView({ block: "center" });
  });
  const issueNow = dataOf(
    await ada.client.GET("/api/v1/issues/{issue}", { params: { path: { issue } } }),
  );
  expect(JSON.stringify(issueNow.issue)).not.toContain(TAG);
  await expect(card).toBeInViewport({ ratio: 1 });
  await good(page, "06-the-rule-proposes");
  await ben.dispose();
  await ada.dispose();
}
