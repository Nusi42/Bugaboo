import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { tool } from "../../../tooling/load/agents.ts";
import { signInThroughApi } from "./accounts.ts";
import { adaOfTheSample, benOfTheSample, shot } from "./demo-m7-common.ts";
import { signIn } from "./flows.ts";
import { sampleAgents } from "./sample-agents.ts";
import { type Instance } from "./server.ts";

// Step 6 of the exit demo of M7 (`m7-exit-demo.spec.ts`): the standing rule of the sample.

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
    value: "from-the-rule",
  });
  expect(proposed.kept).toBe(true);
  await signIn(page, adaOfTheSample());
  await page.goto(issueAddress(issue));
  await expect(page.getByText(/from-the-rule/u).first()).toBeVisible();
  await expect(
    page.getByRole("button", { name: t("agent-work.proposal-apply"), exact: true }),
  ).toBeVisible();
  const issueNow = dataOf(
    await ada.client.GET("/api/v1/issues/{issue}", { params: { path: { issue } } }),
  );
  expect(JSON.stringify(issueNow.issue)).not.toContain("from-the-rule");
  await shot(page, "05-the-rule-proposes");
  await ben.dispose();
  await ada.dispose();
}
