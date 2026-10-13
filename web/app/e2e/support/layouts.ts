import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { roleId, roleName, setFieldLevel, signedInAdmin } from "./access.ts";
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
