import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { ada } from "./accounts.ts";
import { openSession } from "./api.ts";
import { type FakeModel } from "./fake-model.ts";
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
