import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { ada } from "./accounts.ts";
import { sessionOver } from "./api.ts";
import { openFromMenu, signIn } from "./flows.ts";

/** What the server says about the two-step switch of the role whose page is open (the page may not have caught up). */
export async function twoStepOf(page: Page): Promise<unknown> {
  const { origin, pathname } = new URL(page.url());
  const role = pathname.split("/").at(-1) ?? "";
  const session = sessionOver(page.context().request, origin);
  const answer = dataOf(
    await session.client.GET("/api/v1/roles/{role_id}", { params: { path: { role_id: role } } }),
  );
  return answer.role.two_step;
}

/** Makes a role that is a copy of *Viewer* and opens its page: it has ticked boxes and unticked ones. */
export async function copyViewerRole(page: Page, name = "Reviewer"): Promise<void> {
  await openFromMenu(page, t("nav.roles"));
  await page.getByLabel(t("roles.field-name"), { exact: true }).fill(name);
  await page.getByLabel(t("roles.start-from"), { exact: true }).selectOption({
    label: t("roles.start-copy", { name: "Viewer" }),
  });
  await page.getByRole("button", { name: t("roles.create"), exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name })).toBeVisible();
}

/** Signs in as the administrator and opens the page of a new role that is a copy of *Viewer*. */
export async function openRoleWithTickedBox(page: Page, name = "Reviewer"): Promise<void> {
  await signIn(page, ada);
  await copyViewerRole(page, name);
}
