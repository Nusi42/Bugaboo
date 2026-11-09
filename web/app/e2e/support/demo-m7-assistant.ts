import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { signInThroughApi } from "./accounts.ts";
import { ask, assistantPanel, closePanelButton, openAssistant, proposalCard } from "./assistant.ts";
import { adaOfTheSample, good } from "./demo-m7-common.ts";
import { startFakeModel } from "./fake-model.ts";
import { openIssueWithDetails, signIn } from "./flows.ts";
import { type Instance } from "./server.ts";

// Step 2 of the exit demo of M7 (`m7-exit-demo.spec.ts`): the assistant on the sample.

const ISSUE = "WEB-142";

/** The issue of the demo as the person reads it now. */
async function issueNow(session: Awaited<ReturnType<typeof signInThroughApi>>) {
  return dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: ISSUE } } }),
  ).issue;
}

/** Brings the *Priority* of the issue page into view, so a picture shows what is saved. */
async function showsThePriority(page: Page): Promise<void> {
  const label = page.getByText(t("issue-field.priority"), { exact: true }).first();
  await label.scrollIntoViewIfNeeded();
  await expect(label).toBeInViewport();
}

/** Ada asks for a summary, and the picture shows the question and the whole answer. */
async function asksAndShowsTheSummary(
  page: Page,
  panel: ReturnType<typeof assistantPanel>,
): Promise<void> {
  await ask(page, `Summarize ${ISSUE}`);
  await expect(panel.getByText(`Its priority is ${t("priority.normal")}.`)).toBeVisible();
  await showsTheExchange(page, `Summarize ${ISSUE}`);
  await good(page, "02-a-the-summary");
}

/** The panel shows the exchange that starts with `question` from its first line, with no strip of the one before it. */
async function showsTheExchange(page: Page, question: string): Promise<void> {
  await assistantPanel(page)
    .getByText(question, { exact: true })
    .locator("xpath=ancestor::li[1]")
    .evaluate((turn: HTMLElement) => {
      turn.scrollIntoView({ block: "start" });
    });
}

/** The panel is closed and the page shows the priority that was saved. */
async function closesAndShowsThePriority(page: Page): Promise<void> {
  await closePanelButton(page).click();
  await page.getByRole("button", { name: t("assistant.close-yes"), exact: true }).click();
  await expect(assistantPanel(page)).toBeHidden();
  await showsThePriority(page);
  await expect(
    page.getByText(t("priority.critical")).filter({ visible: true }).first(),
  ).toBeVisible();
  await good(page, "02-d-the-issue-after");
}

/** Ada sets up the assistant with the model of this computer and switches AI on. */
async function setsUpTheAssistant(
  session: Awaited<ReturnType<typeof signInThroughApi>>,
  address: string,
): Promise<void> {
  dataOf(
    await session.client.PUT("/api/v1/ai", {
      body: {
        provider: "openai-compatible",
        address,
        model: "fake-model",
        enabled: true,
        password: adaOfTheSample().password,
      },
    }),
  );
}

/** Step 2: the assistant summarizes WEB-142 and proposes a priority that is saved only after Apply. */
export async function theAssistantAnswersAndProposes(page: Page, server: Instance) {
  const model = await startFakeModel();
  const ada = adaOfTheSample();
  const session = await signInThroughApi(server, ada);
  await setsUpTheAssistant(session, model.address);
  const before = await issueNow(session);
  model.will(
    { text: `${ISSUE} is "${before.title}". Its priority is ${t("priority.normal")}.` },
    {
      tool: "update_issue",
      arguments: { issue: ISSUE, version: before.version ?? 1, priority: "critical" },
    },
    { text: "I suggest raising the priority." },
  );
  await signIn(page, ada);
  await openIssueWithDetails(page, ISSUE);
  await showsThePriority(page);
  await good(page, "02-0-the-issue-before");
  const panel = await openAssistant(page);
  await good(page, "02-a-the-panel-opens");
  await asksAndShowsTheSummary(page, panel);
  await ask(page, `Raise the priority of ${ISSUE}.`);
  await expect(proposalCard(page)).toBeVisible();
  const unchanged = await issueNow(session);
  expect(unchanged.priority).toBe(before.priority);
  await showsTheExchange(page, `Raise the priority of ${ISSUE}.`);
  await good(page, "02-b-the-proposal");
  await proposalCard(page)
    .getByRole("button", { name: t("assistant.card.apply") })
    .click();
  await expect(proposalCard(page).getByText(t("assistant.card.saved"))).toBeVisible();
  const saved = await issueNow(session);
  expect(saved.priority).toBe("critical");
  await showsTheExchange(page, `Raise the priority of ${ISSUE}.`);
  await good(page, "02-c-applied");
  await closesAndShowsThePriority(page);
  await model.close();
  await session.dispose();
}
