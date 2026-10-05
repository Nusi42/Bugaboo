import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";
import { expect } from "@playwright/test";

import { type ApiSession } from "./api.ts";
import { layoutOf } from "./layout.ts";

/** The tree of projects: in the sidebar on a computer, in the sheet *More* opens on a phone. */
export async function tree(page: Page): Promise<Locator> {
  if (layoutOf(page) === "phone") {
    const sheet = page.getByRole("dialog", { name: t("shell.more"), exact: true });
    if (!(await sheet.isVisible())) {
      await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
    }
  }
  return page.getByRole("region", { name: t("projects.heading"), exact: true });
}

/** A link of the tree, by the name of the project. */
export async function treeLink(page: Page, name: string): Promise<Locator> {
  return (await tree(page)).getByRole("link", { name: new RegExp(`^${name}`, "u") });
}

/** Makes a project with the form that is open: the name, and the short name as it is typed. */
export async function fillMake(page: Page, name: string, shortName: string): Promise<void> {
  await page.getByLabel(t("projects.field-name"), { exact: true }).fill(name);
  await page.getByLabel(t("projects.field-short-name"), { exact: true }).fill(shortName);
  await page.getByRole("button", { name: t("projects.make"), exact: true }).click();
  await expect(page.getByText(t("projects.made", { name }))).toBeVisible();
}

/** Makes a project through the API, at the top level or inside another, and returns its ID. */
export async function makeProject(
  admin: ApiSession,
  shortName: string,
  name: string,
  inside?: string,
): Promise<string> {
  const made = await admin.post(
    inside === undefined ? "/api/v1/projects" : `/api/v1/projects/${inside}/sub-projects`,
    { short_name: shortName, name },
  );
  return String(made["id"]);
}

/** Opens *Move* on the settings page at `address`, picks the top level and waits for the preview. */
export async function openMove(page: Page, address: string): Promise<Locator> {
  await page.goto(address);
  await page.getByRole("button", { name: t("projects.move-button"), exact: true }).click();
  const dialog = page.getByRole("alertdialog");
  await expect(dialog).toBeVisible();
  await dialog.getByRole("radio", { name: t("projects.picker-top") }).check();
  await expect(dialog.getByRole("heading", { name: t("projects.preview-heading") })).toBeVisible();
  return dialog;
}

/** What the project pages show on a computer includes the tree, which a phone keeps in a sheet. */
export async function openOverview(page: Page, address: string, name: string): Promise<void> {
  await page.goto(address);
  await expect(page.getByRole("heading", { level: 1, name })).toBeVisible();
  if (layoutOf(page) === "computer") {
    await tree(page);
  }
}
