import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { ada } from "./accounts.ts";
import { openSession } from "./api.ts";
import { type FakeModel } from "./fake-model.ts";
import { box, dialog } from "./quick-search.ts";
import { type Instance } from "./server.ts";

/** An administrator sets up the assistant with the fake model of a test and switches AI on. */
export async function setUpAssistant(server: Instance, model: FakeModel): Promise<void> {
  const admin = openSession(server.url);
  await admin.client.POST("/api/v1/auth/sign-in", {
    body: { email: ada.email, password: ada.password },
  });
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
  const admin = openSession(server.url);
  await admin.client.POST("/api/v1/auth/sign-in", {
    body: { email: ada.email, password: ada.password },
  });
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
  const admin = openSession(server.url);
  await admin.client.POST("/api/v1/auth/sign-in", {
    body: { email: ada.email, password: ada.password },
  });
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: key } },
    body: { version: now.version, priority: "normal" },
  });
  await admin.dispose();
}
