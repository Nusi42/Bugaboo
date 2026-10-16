import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { type ApiSession } from "./api.ts";
import { tickBox } from "./issues.ts";
import { layoutOf } from "./layout.ts";
import { projectIssuesAddress } from "../../src/shared/paths.ts";

/** Makes a label through the API and returns its ID. */
export async function makeLabel(admin: ApiSession, name: string): Promise<string> {
  return dataOf(await admin.client.POST("/api/v1/labels", { body: { name } })).id;
}

/** The ID of the label with this name. */
export async function labelIdOf(admin: ApiSession, name: string): Promise<string> {
  const found = dataOf(await admin.client.GET("/api/v1/labels")).labels.find(
    (label) => label.name === name,
  );
  if (found === undefined) {
    throw new TypeError(`The server has no label ${name}.`);
  }
  return found.id;
}

/** Opens the details of the issue where they sit behind *All details* (a smaller screen). */
export async function showDetails(page: Page): Promise<void> {
  if (layoutOf(page) !== "computer") {
    await page.getByRole("button", { name: t("issues.all-details"), exact: true }).click();
  }
}

/** The names of the labels an issue has, as the API says it. */
export async function labelNamesOf(admin: ApiSession, issue: string): Promise<string[]> {
  const sorting = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}/sorting", { params: { path: { issue } } }),
  );
  return (sorting.labels ?? []).map((label) => label.name);
}

/** Opens the list of the project WEB and ticks its first two issues. */
export async function tickTwo(page: Page): Promise<void> {
  await page.goto(projectIssuesAddress("WEB"));
  await tickBox(page, "WEB-1").check({ force: true });
  await tickBox(page, "WEB-2").check({ force: true });
  await expect(
    page.getByRole("status").filter({ hasText: t("issues.selected", { count: 2 }) }),
  ).toBeVisible();
}
