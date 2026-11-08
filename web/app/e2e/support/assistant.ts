import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { ada, signInThroughApi } from "./accounts.ts";
import { type FakeModel } from "./fake-model.ts";
import { layoutOf } from "./layout.ts";
import { box, dialog } from "./quick-search.ts";
import { type Instance } from "./server.ts";

/** An administrator sets up the assistant with the fake model of a test and switches AI on. */
export async function setUpAssistant(server: Instance, model: FakeModel): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  const changed = await admin.client.PUT("/api/v1/ai", {
    body: {
      provider: "openai-compatible",
      address: model.address,
      model: "fake-model",
      enabled: true,
      password: ada.password,
    },
  });
  if (!changed.response.ok) {
    throw new Error(`the assistant could not be set up: ${String(changed.response.status)}`);
  }
  await admin.dispose();
}

/** The priority of an issue and the version it is at, as the server has them now. */
export async function priorityAndVersion(
  server: Instance,
  key: string,
): Promise<{ readonly priority: string; readonly version: number }> {
  const admin = await signInThroughApi(server, ada);
  const found = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  await admin.dispose();
  return { priority: found.issue.priority ?? "", version: found.issue.version ?? 1 };
}

/** The button of the top bar that opens the assistant. */
export function assistantButton(page: Page) {
  return page.getByRole("button", { name: t("assistant.button"), exact: true });
}

/** The panel of the assistant. */
export function assistantPanel(page: Page) {
  return page.getByRole("dialog", { name: t("assistant.title") });
}

/** Opens the panel from the top bar. */
export async function openAssistant(page: Page) {
  await assistantButton(page).click();
  const panel = assistantPanel(page);
  await expect(panel).toBeVisible();
  return panel;
}

/** Types a question into the open panel and sends it. */
export async function ask(page: Page, question: string): Promise<void> {
  const panel = assistantPanel(page);
  await panel.getByRole("textbox", { name: t("assistant.question") }).fill(question);
  await panel.getByRole("button", { name: t("assistant.ask"), exact: true }).click();
}

/** Opens quick search, looks for the assistant, and gives the entries that offer it. */
export async function quickEntryOfTheAssistant(page: Page) {
  await page.keyboard.press("Control+k");
  await expect(dialog(page)).toBeVisible();
  await box(page).fill(t("assistant.quick-ask"));
  return dialog(page).getByRole("option", { name: t("assistant.quick-ask") });
}

/** The model proposes to raise the priority of an issue to Critical and then says `said`; gives the issue as it is now. */
export async function proposeCritical(
  server: Instance,
  model: FakeModel,
  key: string,
  said: string,
): Promise<{ readonly priority: string; readonly version: number }> {
  const before = await priorityAndVersion(server, key);
  model.will(
    {
      tool: "update_issue",
      arguments: { issue: key, version: before.version, priority: "critical" },
    },
    { text: said },
  );
  return before;
}

/** Puts the priority of an issue back to Normal, so a picture of a proposal to raise it can be taken again. */
export async function lowerToNormal(server: Instance, key: string): Promise<void> {
  const now = await priorityAndVersion(server, key);
  const admin = await signInThroughApi(server, ada);
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: key } },
    body: { version: now.version, priority: "normal" },
  });
  await admin.dispose();
}

/** An administrator switches AI off for the organization again, so a test can set it up from the start. */
export async function switchAiOff(server: Instance): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  await admin.client.PUT("/api/v1/ai", { body: { enabled: false } });
  await admin.dispose();
}

/** Flips a switch by pressing its words, as a person does (the switch itself is a hidden box). */
export async function flip(page: Page, name: string): Promise<void> {
  await page.getByRole("switch", { name }).evaluate((input: HTMLElement) => {
    input.scrollIntoView({ block: "center" });
  });
  await page.getByText(name, { exact: true }).first().click();
}

/** Somebody else changes the title of an issue, through the API, on the version it has now. */
export async function changeTitleBehindTheScenes(
  server: Instance,
  key: string,
  title: string,
): Promise<void> {
  const now = await priorityAndVersion(server, key);
  const admin = await signInThroughApi(server, ada);
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: key } },
    body: { version: now.version, title },
  });
  await admin.dispose();
}

/** An administrator sets up an outside provider, confirms the agreement and switches AI on; nothing is called. */
export async function setUpOutsideProvider(server: Instance): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  await admin.client.PUT("/api/v1/ai", {
    body: {
      enabled: false,
      provider: "anthropic",
      address: "",
      model: "claude-test",
      password: ada.password,
    },
  });
  await admin.client.POST("/api/v1/ai/agreement", { body: { destination: "anthropic" } });
  await admin.client.PUT("/api/v1/ai", { body: { enabled: true } });
  await admin.dispose();
}

/** An administrator marks a built-in field Never sent to AI. */
export async function markFieldNeverSent(server: Instance, field: string): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  await admin.client.PUT("/api/v1/never-sent-to-ai/fields/{field}", {
    params: { path: { field } },
    body: { marked: true, version: 0 },
  });
  await admin.dispose();
}

/** An administrator switches AI off for one project (`false`), or lets it follow the organization again (`true`). */
export async function setProjectAi(
  server: Instance,
  project: string,
  followsOrganization: boolean,
): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  await admin.client.PUT("/api/v1/projects/{project}/ai", {
    params: { path: { project } },
    body: followsOrganization ? {} : { enabled: false },
  });
  await admin.dispose();
}

/** The comments of an issue as safe HTML, as the history has them now (a comment in the trash is not in it). */
export async function commentTexts(server: Instance, key: string): Promise<readonly string[]> {
  const admin = await signInThroughApi(server, ada);
  const found = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}/history", {
      params: { path: { issue: key }, query: { tab: "comments", fields: "comment,html" } },
    }),
  );
  await admin.dispose();
  const texts: string[] = [];
  for (const each of found.items) {
    texts.push(JSON.stringify(each.html ?? ""));
  }
  return texts;
}

/** The button that closes the open panel: *Close* on a computer and a tablet, *Back* on a phone (UX §6.3). */
export function closePanelButton(page: Page) {
  const name = layoutOf(page) === "phone" ? t("shell.back") : t("assistant.close");
  return assistantPanel(page).getByRole("button", { name, exact: true });
}

/** The card of a proposed change in the open panel. */
export function proposalCard(page: Page) {
  return assistantPanel(page).getByRole("region", { name: t("assistant.card.title") });
}

/** Chooses the outside provider on the set-up page and saves it. */
export async function saveOutsideProvider(page: Page): Promise<void> {
  await page.getByLabel(t("setting.ai.provider.name"), { exact: true }).selectOption({
    label: t("ai.provider.anthropic"),
  });
  await page.getByLabel(t("setting.ai.model.name"), { exact: true }).fill("claude-test");
  await page.getByLabel(t("ai-setup.password")).fill(ada.password);
  await page.getByRole("button", { name: t("ai-setup.save") }).click();
  await expect(page.getByText(t("ai-setup.saved"))).toBeVisible();
}

/**
 * Waits for the answer that says whether the assistant is at this place: a page where it is off shows no
 * button either while that answer is still on its way, so a check that nothing is there counts only after it.
 */
export function assistantStatusAnswered(page: Page): Promise<unknown> {
  return page.waitForResponse(
    (answer) =>
      new URL(answer.url()).pathname === "/api/v1/assistant" && answer.request().method() === "GET",
  );
}
