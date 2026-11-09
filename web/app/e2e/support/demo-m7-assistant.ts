import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { signInThroughApi } from "./accounts.ts";
import { ask, openAssistant, proposalCard } from "./assistant.ts";
import { adaOfTheSample, good } from "./demo-m7-common.ts";
import { startFakeModel } from "./fake-model.ts";
import { signIn } from "./flows.ts";
import { type Instance } from "./server.ts";

// Step 2 of the exit demo of M7 (`m7-exit-demo.spec.ts`): the assistant on the sample.

const ISSUE = "WEB-142";

/** Step 2: the assistant summarizes WEB-142 and proposes a priority that is saved only after Apply. */
/** The issue of the demo as the person reads it now. */
async function issueNow(session: Awaited<ReturnType<typeof signInThroughApi>>) {
  return dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: ISSUE } } }),
  ).issue;
}

export async function theAssistantAnswersAndProposes(page: Page, server: Instance) {
  const model = await startFakeModel();
  const ada = adaOfTheSample();
  const session = await signInThroughApi(server, ada);
  dataOf(
    await session.client.PUT("/api/v1/ai", {
      body: {
        provider: "openai-compatible",
        address: model.address,
        model: "fake-model",
        enabled: true,
        password: ada.password,
      },
    }),
  );
  const before = await issueNow(session);
  model.will(
    { text: "The issue asks for a fix of the login page. Nobody has started yet." },
    {
      tool: "update_issue",
      arguments: { issue: ISSUE, version: before.version ?? 1, priority: "critical" },
    },
    { text: "I suggest raising the priority." },
  );
  await signIn(page, ada);
  await page.goto(issueAddress(ISSUE));
  const panel = await openAssistant(page);
  await ask(page, `Summarize ${ISSUE}`);
  await expect(panel.getByText("Nobody has started yet.")).toBeVisible();
  await good(page, "02-a-the-summary");
  await ask(page, `Raise the priority of ${ISSUE}.`);
  await expect(proposalCard(page)).toBeVisible();
  const unchanged = await issueNow(session);
  expect(unchanged.priority).toBe(before.priority);
  await good(page, "02-b-the-proposal");
  await proposalCard(page)
    .getByRole("button", { name: t("assistant.card.apply") })
    .click();
  await expect(proposalCard(page).getByText(t("assistant.card.saved"))).toBeVisible();
  const saved = await issueNow(session);
  expect(saved.priority).toBe("critical");
  await good(page, "02-c-applied");
  await model.close();
  await session.dispose();
}
