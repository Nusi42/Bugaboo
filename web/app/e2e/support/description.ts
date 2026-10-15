import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";

/** The box the description is written in. */
export function descriptionBox(page: Page) {
  return page.getByRole("textbox", { name: t("issue-field.description"), exact: true });
}

/** The button that saves what is being edited. */
export function saveButton(page: Page) {
  return page.getByRole("button", { name: t("common.save"), exact: true });
}

/** The button that saves the description, on a page where other fields have a *Save* of their own. */
export function saveDescriptionButton(page: Page) {
  // The nearest box around the text box that has a Save: the editor of the description. A box
  // is found by what it holds (a role and a name), so no text is put into a path.
  const save = page.getByRole("button", { name: t("common.save"), exact: true });
  return page
    .locator("div, form, section")
    .filter({ has: descriptionBox(page) })
    .filter({ has: save })
    .last()
    .getByRole("button", { name: t("common.save"), exact: true });
}

/** The button of a text that was not saved because the issue changed: *Compare and save*. */
export function compareAndSaveButton(page: Page) {
  return page.getByRole("button", { name: t("issues.compare-and-save"), exact: true });
}

/** Opens the description of an issue for editing; the box is there unless somebody else edits. */
export async function openEditing(page: Page, key: string): Promise<void> {
  await page.goto(issueAddress(key));
  await page.getByRole("button", { name: t("issues.description.edit"), exact: true }).click();
}

/** Opens the box for the description of an issue. */
export async function startEditing(page: Page, key: string): Promise<void> {
  await openEditing(page, key);
  await expect(descriptionBox(page)).toBeVisible();
}

/**
 * Types into the box and waits until the server has given the lock: the first key asks for it,
 * and what other people see (*… is editing*) only starts when it is given.
 */
export async function typeUnderLock(page: Page, text: string): Promise<void> {
  const taken = page.waitForResponse(
    (answer) => answer.request().method() === "POST" && answer.url().endsWith("/api/v1/edit-locks"),
  );
  await descriptionBox(page).fill(text);
  expect((await taken).ok()).toBe(true);
}

/** The note that says who is editing the description. */
export function isEditing(page: Page, name: string) {
  return page.getByText(new RegExp(`^${name} is editing the description`, "u"));
}

/** The page tells who is editing and gives no box to type in. */
export async function expectToldWhoIsEditing(page: Page, name: string): Promise<void> {
  await expect(isEditing(page, name)).toBeVisible();
  await expect(descriptionBox(page)).toHaveCount(0);
}
