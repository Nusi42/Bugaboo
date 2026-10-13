import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Browser, type Locator, type Page } from "@playwright/test";

import { addressInProject, paths } from "../../src/shared/paths.ts";
import { roleId, roleName, setFieldLevel, signedInAdmin } from "./access.ts";
import { ada } from "./accounts.ts";
import { anotherBrowser, signIn } from "./flows.ts";
import { prepareWeb } from "./issues.ts";
import { type Instance } from "./server.ts";

/** The name of the own field that the tests of layouts move around. */
export const dueDate = "Due date";

/**
 * Gets a server ready for the tests of layouts: the project WEB with a Team member (Bob), a Guest
 * (Lea) and a Viewer (Ana), and the own field *Due date* for issues, which the Guest may fill in
 * (so only the layout decides whether the form asks the Guest for it).
 */
export async function prepareLayouts(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  const made = dataOf(
    await admin.client.POST("/api/v1/fields", {
      body: { name: dueDate, kind: "date", applies_to: "issues" },
    }),
  );
  await setFieldLevel(
    admin,
    { role: await roleId(admin, roleName.guest), field: made.id },
    "can-change",
  );
  await admin.dispose();
}

/**
 * Waits until the editor of a layout can be used: when another window of the same person still
 * holds the lock (the lease lasts a minute after a window closed), takes the editing over.
 */
export async function editWhenYouCan(page: Page, ready: Locator): Promise<void> {
  const takeOver = page.getByRole("button", { name: t("layouts.edit-anyway"), exact: true });
  // The editor is on its way (its buttons are switched off) until the lock is decided.
  await expect
    .poll(async () => (await takeOver.isVisible()) || (await ready.isEnabled()))
    .toBe(true);
  if (await takeOver.isVisible()) {
    await takeOver.click();
  }
  await expect(ready).toBeEnabled();
}

/** Adds a section to the layout in the editor by pressing the button with the keyboard. */
export async function addSectionWithKeyboard(page: Page): Promise<void> {
  await page.getByRole("button", { name: t("layouts.add-section"), exact: true }).focus();
  await page.keyboard.press("Enter");
}

/** Picks a choice of a list in the editor with the keyboard alone: the list has the focus and the choice is typed. */
export async function chooseByTyping(page: Page, list: Locator, choice: string): Promise<void> {
  await list.focus();
  await page.keyboard.type(choice);
}

/**
 * The same person, in another window, opens the layout of *Bug*, takes the editing over, adds a
 * section with this title and saves.
 */
export async function saveSectionInAnotherWindow(
  browser: Browser,
  server: Instance,
  title: string,
): Promise<void> {
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await openLayoutOfBugs(other);
  const add = other.getByRole("button", { name: t("layouts.add-section"), exact: true });
  await editWhenYouCan(other, add);
  await add.click();
  await other.getByLabel(t("layouts.section-title"), { exact: true }).last().fill(title);
  await other.getByRole("button", { name: t("layouts.save"), exact: true }).click();
  await expect(other.getByText(t("layouts.saved"), { exact: true })).toBeVisible();
}

/** Opens the editor of the new-issue form for the issue type *Bug*, which the dialog customized. */
export async function openLayoutOfBugs(page: Page): Promise<void> {
  await page.goto(addressInProject(paths.projectLayouts, "WEB"));
  await page.getByLabel(t("layouts.type"), { exact: true }).selectOption({ label: "Bug" });
}
